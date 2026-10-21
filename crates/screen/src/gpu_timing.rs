//! Opt-in render-pass timestamps. Three reusable readbacks bound in-flight work;
//! polling never waits, and busy/failed/exhausted samples never delay rendering.
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use std::time::Duration;

const SLOTS: usize = 3;
pub(crate) const SAMPLE_LIMIT: u64 = 18_000;
const IDLE: u8 = 0;
const MAPPING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

#[derive(Clone, Copy, Default, Debug)]
pub struct GpuTimingCounts {
    pub submitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub invalid: u64,
    pub poll_errors: u64,
    pub busy_skipped: u64,
    pub limit_skipped: u64,
    /// Current gauge, not an increment. Shutdown does not wait for these samples.
    pub pending: u64,
}

/// Durations belong to earlier unpaused render passes, not the current CPU frame.
#[derive(Default)]
pub struct GpuTimings {
    pub samples: [Option<Duration>; SLOTS],
    /// Counts since the preceding drain, except the absolute pending gauge.
    pub counts: GpuTimingCounts,
}

struct Slot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

pub(crate) struct GpuProfiler {
    query: wgpu::QuerySet,
    slots: [Slot; SLOTS],
    period: f32,
    submitted: u64,
    limit: u64,
    stopped: bool,
    batch: GpuTimings,
}

pub(crate) fn requested_features(requested: bool, available: wgpu::Features) -> wgpu::Features {
    if requested {
        available & wgpu::Features::TIMESTAMP_QUERY
    } else {
        wgpu::Features::empty()
    }
}

fn duration(begin: u64, end: u64, period: f32) -> Option<Duration> {
    if !period.is_finite() || period <= 0.0 {
        return None;
    }
    let ticks = end.checked_sub(begin)?;
    Duration::try_from_secs_f64(ticks as f64 * period as f64 * 1e-9).ok()
}

impl GpuProfiler {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            query: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("screen-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: (SLOTS * 2) as u32,
            }),
            slots: std::array::from_fn(|_| Slot {
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("screen-timestamp-resolve"),
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("screen-timestamp-readback"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(IDLE)),
            }),
            period: queue.get_timestamp_period(),
            submitted: 0,
            limit: SAMPLE_LIMIT,
            stopped: false,
            batch: GpuTimings::default(),
        }
    }

    pub(crate) fn poll(&mut self, device: &wgpu::Device) {
        if self.stopped
            || self
                .slots
                .iter()
                .all(|s| s.state.load(Ordering::Acquire) == IDLE)
        {
            return;
        }
        if device.poll(wgpu::PollType::Poll).is_err() {
            // Stop diagnostic work only; normal presentation owns device recovery.
            self.batch.counts.poll_errors += 1;
            self.stopped = true;
            return;
        }
        for (index, slot) in self.slots.iter().enumerate() {
            match slot.state.load(Ordering::Acquire) {
                READY => {
                    let bytes = slot.readback.slice(..).get_mapped_range();
                    let begin = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
                    let end = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
                    let sample = duration(begin, end, self.period);
                    drop(bytes);
                    slot.readback.unmap();
                    if let Some(sample) = sample {
                        self.batch.samples[index] = Some(sample);
                        self.batch.counts.completed += 1;
                    } else {
                        self.batch.counts.invalid += 1;
                    }
                    slot.state.store(IDLE, Ordering::Release);
                }
                FAILED => {
                    slot.readback.unmap();
                    self.batch.counts.failed += 1;
                    // A mapping failure can be device loss; don't retry indefinitely.
                    self.stopped = true;
                    slot.state.store(IDLE, Ordering::Release);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn reserve(&mut self) -> Option<usize> {
        if self.stopped {
            return None;
        }
        if self.submitted >= self.limit {
            self.batch.counts.limit_skipped += 1;
            return None;
        }
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.state.load(Ordering::Acquire) == IDLE)
        else {
            self.batch.counts.busy_skipped += 1;
            return None;
        };
        self.slots[index].state.store(MAPPING, Ordering::Release);
        self.submitted += 1;
        self.batch.counts.submitted += 1;
        Some(index)
    }

    pub(crate) fn writes(&self, index: usize) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.query,
            beginning_of_pass_write_index: Some((index * 2) as u32),
            end_of_pass_write_index: Some((index * 2 + 1) as u32),
        }
    }

    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder, index: usize) {
        let slot = &self.slots[index];
        encoder.resolve_query_set(
            &self.query,
            (index * 2) as u32..(index * 2 + 2) as u32,
            &slot.resolve,
            0,
        );
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, 16);
    }

    pub(crate) fn map(&self, index: usize) {
        let slot = &self.slots[index];
        let state = Arc::clone(&slot.state);
        slot.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() { READY } else { FAILED },
                    Ordering::Release,
                );
            });
    }

    pub(crate) fn take(&mut self) -> GpuTimings {
        self.batch.counts.pending = self
            .slots
            .iter()
            .filter(|s| s.state.load(Ordering::Acquire) != IDLE)
            .count() as u64;
        std::mem::take(&mut self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_are_optional_and_timestamp_conversion_rejects_invalid_intervals() {
        let all = wgpu::Features::all();
        assert_eq!(requested_features(false, all), wgpu::Features::empty());
        assert_eq!(
            requested_features(true, wgpu::Features::empty()),
            wgpu::Features::empty()
        );
        assert_eq!(
            requested_features(true, all),
            wgpu::Features::TIMESTAMP_QUERY
        );
        assert_eq!(
            duration(1_000, 1_500, 2.0),
            Some(Duration::from_nanos(1_000))
        );
        assert_eq!(duration(10, 10, 1.0), Some(Duration::ZERO));
        for period in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(duration(0, 1, period).is_none());
        }
        assert!(duration(5, 4, 1.0).is_none());
        assert!(duration(0, u64::MAX, f32::MAX).is_none());
    }
    #[test]
    #[ignore = "requires a working timestamp-capable graphics adapter; run explicitly"]
    fn deferred_readback_bounds_work_and_preserves_rendered_pixels() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        eprintln!("timestamp test adapter: {:?}", adapter.get_info());
        assert!(adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY));
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            ..Default::default()
        }))
        .unwrap();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let draw = |profile: Option<(&GpuProfiler, usize)>| {
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.25,
                                g: 0.5,
                                b: 0.75,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    timestamp_writes: profile.map(|(p, i)| p.writes(i)),
                    ..Default::default()
                });
            }
            if let Some((p, i)) = profile {
                p.resolve(&mut encoder, i);
            }
            queue.submit([encoder.finish()]);
            if let Some((p, i)) = profile {
                p.map(i);
            }
        };
        let read_pixels = || {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 512,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(2),
                    },
                },
                wgpu::Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            let (sender, receiver) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    sender.send(result).unwrap()
                });
            // Blocking is confined to this explicit correctness test.
            device.poll(wgpu::PollType::Wait).unwrap();
            receiver.recv().unwrap().unwrap();
            let data = buffer.slice(..).get_mapped_range();
            let pixels = [data[..8].to_vec(), data[256..264].to_vec()].concat();
            drop(data);
            buffer.unmap();
            pixels
        };
        draw(None);
        let reference = read_pixels();
        assert_eq!(&reference[..4], &[64, 128, 191, 255]);
        let mut profile = GpuProfiler::new(&device, &queue);
        profile.limit = 4;
        for _ in 0..SLOTS {
            let index = profile.reserve().unwrap();
            draw(Some((&profile, index)));
        }
        // Even already-ready callbacks cannot free a slot before it is drained.
        assert!(profile.reserve().is_none());
        device.poll(wgpu::PollType::Wait).unwrap();
        profile.poll(&device);
        let batch = profile.take();
        assert_eq!(
            (
                batch.counts.submitted,
                batch.counts.completed,
                batch.counts.busy_skipped,
                batch.counts.pending
            ),
            (3, 3, 1, 0)
        );
        assert!(batch.samples.iter().all(Option::is_some));
        assert_eq!(
            (
                batch.counts.failed,
                batch.counts.invalid,
                batch.counts.poll_errors
            ),
            (0, 0, 0)
        );
        assert_eq!(read_pixels(), reference);
        let index = profile.reserve().unwrap();
        draw(Some((&profile, index)));
        assert!(profile.reserve().is_none());
        device.poll(wgpu::PollType::Wait).unwrap();
        profile.poll(&device);
        let batch = profile.take();
        assert_eq!(
            (
                batch.counts.submitted,
                batch.counts.completed,
                batch.counts.limit_skipped,
                batch.counts.pending
            ),
            (1, 1, 1, 0)
        );
        assert_eq!(read_pixels(), reference);
        assert_eq!(profile.take().counts.completed, 0);
    }
}
