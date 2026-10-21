//! Opt-in, bounded frame-time capture. No clock reads or allocations when off.
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 360_000;
const FRAME_BUDGET: Duration = Duration::from_nanos(16_742_706);

pub struct FrameTimes {
    enabled: bool,
    samples: Vec<Duration>,
    omitted: u64,
    limit: usize,
}

impl FrameTimes {
    pub fn new(enabled: bool) -> Self {
        Self::with_limit(enabled, MAX_SAMPLES)
    }

    fn with_limit(enabled: bool, limit: usize) -> Self {
        Self {
            enabled,
            // Reserve outside the measured loop; capture itself never reallocates.
            samples: if enabled {
                Vec::with_capacity(limit)
            } else {
                Vec::new()
            },
            omitted: 0,
            limit,
        }
    }

    pub fn start(&self, measuring: bool) -> Option<Instant> {
        (self.enabled && measuring).then(Instant::now)
    }

    pub fn finish(&mut self, start: Option<Instant>) {
        if let Some(start) = start {
            self.record(start.elapsed());
        }
    }

    fn record(&mut self, duration: Duration) {
        if self.samples.len() < self.limit {
            self.samples.push(duration);
        } else {
            self.omitted += 1;
        }
    }

    /// Report after timing ends; sorting/logging must not enter the frame loop.
    pub fn report(&mut self) {
        self.report_named("FFTA_FRAME_TIMES");
    }

    fn report_named(&mut self, prefix: &str) {
        if !self.enabled || self.samples.is_empty() {
            return;
        }
        self.samples.sort_unstable();
        let n = self.samples.len();
        let percentile =
            |p: usize| self.samples[(n * p).div_ceil(100).saturating_sub(1)].as_secs_f64() * 1000.0;
        let over_budget = self.samples.iter().filter(|&&d| d > FRAME_BUDGET).count();
        let over_double = self
            .samples
            .iter()
            .filter(|&&d| d > FRAME_BUDGET * 2)
            .count();
        let total_ms = self.samples.iter().map(Duration::as_secs_f64).sum::<f64>() * 1000.0;
        println!(
            "{prefix} samples={n} omitted={} p50_ms={:.6} p95_ms={:.6} p99_ms={:.6} max_ms={:.6} over_budget={over_budget} over_double_budget={over_double} total_ms={total_ms:.6} mean_ms={:.6}",
            self.omitted, percentile(50), percentile(95), percentile(99), percentile(100), total_ms / n as f64,
        );
    }
}

/// Per-frame PPU wall time inside core execution. Parallel zoom includes the
/// caller's wait for workers; durations are not summed CPU usage across threads.
pub struct PpuProfile {
    detailed: bool,
    stages: [FrameTimes; 6],
    series: [FrameTimes; 3],
    frames: u64,
    scanlines: u64,
    zoom: u64,
    invalid: u64,
}

impl PpuProfile {
    pub fn new(enabled: bool) -> Self {
        Self::configured(enabled, MAX_SAMPLES)
    }

    fn configured(enabled: bool, limit: usize) -> Self {
        let detailed = enabled && std::env::var("FFTA_PPU_STAGES").as_deref() == Ok("1");
        Self::with_detail_limit(enabled, detailed, limit)
    }

    #[cfg(test)]
    fn with_limit(enabled: bool, limit: usize) -> Self {
        Self::with_detail_limit(enabled, false, limit)
    }

    fn with_detail_limit(enabled: bool, detailed: bool, limit: usize) -> Self {
        let detailed = enabled && detailed;
        Self {
            detailed,
            stages: std::array::from_fn(|_| FrameTimes::with_limit(detailed, limit)),
            series: std::array::from_fn(|_| FrameTimes::with_limit(enabled, limit)),
            frames: 0,
            scanlines: 0,
            zoom: 0,
            invalid: 0,
        }
    }

    pub fn enabled(&self) -> bool {
        self.series[0].enabled
    }

    pub fn detailed(&self) -> bool {
        self.detailed
    }

    pub fn record(&mut self, timings: Option<gba_core::ppu_profile::Timings>) {
        if !self.enabled() {
            return;
        }
        if let Some(t) = timings {
            let ppu = t.scanlines + t.zoom;
            self.frames += 1;
            self.scanlines += t.scanline_calls as u64;
            self.zoom += t.zoom_calls as u64;
            let stage_total: Duration = t.stages.iter().sum();
            self.invalid += u64::from(
                ppu > t.frame || (self.detailed && (!t.detailed || stage_total > t.scanlines)),
            );
            if self.detailed {
                for (series, duration) in self.stages.iter_mut().zip(
                    t.stages
                        .into_iter()
                        .chain([t.scanlines.saturating_sub(stage_total)]),
                ) {
                    series.record(duration);
                }
            }
            self.series[0].record(t.scanlines);
            self.series[1].record(t.zoom);
            self.series[2].record(t.frame.saturating_sub(ppu));
        }
    }

    pub fn report(&mut self) {
        if !self.enabled() {
            return;
        }
        println!(
            "FFTA_PPU_COUNTS frames={} scanlines={} zoom={} invalid={}",
            self.frames, self.scanlines, self.zoom, self.invalid
        );
        for (series, name) in
            self.series
                .iter_mut()
                .zip(["hardware_scanlines", "zoom_rows", "core_other"])
        {
            series.report_named(&format!("FFTA_PPU_FRAME phase={name}"));
        }
        if self.detailed {
            for (series, name) in self.stages.iter_mut().zip(
                gba_core::ppu_profile::SCANLINE_STAGES
                    .into_iter()
                    .chain(["scanline_other"]),
            ) {
                series.report_named(&format!("FFTA_PPU_STAGE phase={name}"));
            }
        }
    }
}

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Phase {
    Core,
    UiPrepare,
    TemporalPush,
    TemporalCompose,
    AudioHandoff,
    Autosave,
    Present,
    PacingWait,
    Tick,
    UiUpload,
    GpuSetupUpload,
    GpuAcquire,
    GpuEncode,
    GpuSubmit,
    GpuPresent,
    GpuReadback,
}

const PHASE_NAMES: [&str; 16] = [
    "core_including_ppu",
    "ui_prepare",
    "temporal_push",
    "temporal_compose",
    "audio_handoff",
    "autosave",
    "present_including_acquire_upload_submit",
    "pacing_wait",
    "tick_after_input_including_work_and_wait",
    "ui_layout_upload",
    "gpu_cpu_setup_upload",
    "gpu_cpu_acquire",
    "gpu_cpu_encode",
    "gpu_cpu_submit",
    "gpu_cpu_present",
    "gpu_cpu_timestamp_readback",
];

/// CPU wall times, with explicit inclusive scopes. This does not measure GPU
/// execution. The core includes PPU work; a tick includes the other stages.
pub struct PlayProfile {
    series: [FrameTimes; PHASE_NAMES.len()],
    ui_payload_bytes: u64,
    ui_updates: u64,
    ppu: PpuProfile,
    gpu: FrameTimes,
    gpu_counts: screen::present::GpuTimingCounts,
    measuring: bool,
    cadence: Option<PresentationCadence>,
}

impl PlayProfile {
    pub fn new(enabled: bool) -> Self {
        Self {
            series: std::array::from_fn(|_| FrameTimes::with_limit(enabled, 18_000)),
            ui_payload_bytes: 0,
            ui_updates: 0,
            ppu: PpuProfile::configured(enabled, 18_000),
            gpu: FrameTimes::with_limit(enabled && std::env::var("FFTA_GPU_TIMESTAMPS").as_deref() == Ok("1"), 18_000),
            gpu_counts: Default::default(),
            measuring: true,
            cadence: None,
        }
    }

    pub fn start(&self) -> Option<Instant> {
        self.series[0].start(self.measuring)
    }

    pub fn set_measuring(&mut self, measuring: bool) { self.measuring = measuring; }

    pub fn configure_cadence(&mut self, window: Option<&str>) -> Result<(), String> {
        if self.series[0].enabled {
            self.cadence = Some(PresentationCadence::new(window, 18_000)?);
        }
        Ok(())
    }

    pub fn prepare_presentation(&mut self, frame: u64, eligible: bool) {
        if let Some(c) = &mut self.cadence { c.prepare(frame, eligible); }
    }

    pub fn presentation_boundary(&mut self) {
        if let Some(c) = &mut self.cadence { c.last = None; }
    }

    pub fn presentation_restored(&mut self, frame: u64) {
        if let Some(c) = &mut self.cadence {
            c.last = None;
            if let Some(w) = &mut c.window { w.restore(frame); }
        }
    }

    pub fn presentation_skipped(&mut self) {
        if let Some(c) = &mut self.cadence {
            if c.active { c.skipped += 1; }
        }
    }

    /// Reuse the FPS meter's timestamp, taken after successful host presentation.
    pub fn presented(&mut self, now: Instant, software: bool) {
        if let Some(c) = &mut self.cadence { c.presented(now, software); }
    }

    pub fn enabled(&self) -> bool {
        self.series[0].enabled && self.measuring
    }

    pub fn gpu_enabled(&self) -> bool { self.gpu.enabled }

    pub fn record_gpu(&mut self, batch: screen::present::GpuTimings) {
        if !self.gpu.enabled { return; }
        for duration in batch.samples.into_iter().flatten() { self.gpu.record(duration); }
        let c = batch.counts;
        self.gpu_counts.submitted += c.submitted;
        self.gpu_counts.completed += c.completed;
        self.gpu_counts.failed += c.failed;
        self.gpu_counts.invalid += c.invalid;
        self.gpu_counts.poll_errors += c.poll_errors;
        self.gpu_counts.busy_skipped += c.busy_skipped;
        self.gpu_counts.limit_skipped += c.limit_skipped;
        self.gpu_counts.pending = c.pending;
    }

    pub fn detailed_ppu(&self) -> bool {
        self.ppu.detailed()
    }

    pub fn record_ppu(&mut self, timings: Option<gba_core::ppu_profile::Timings>) {
        self.ppu.record(timings);
    }

    pub fn record_ui_upload(&mut self, bytes: usize) {
        if self.enabled() {
            self.ui_payload_bytes += bytes as u64;
            self.ui_updates += 1;
        }
    }

    pub fn record(&mut self, phase: Phase, duration: Option<Duration>) {
        if self.enabled() {
            if let Some(duration) = duration {
                self.series[phase as usize].record(duration);
            }
        }
    }

    pub fn finish(&mut self, phase: Phase, start: Option<Instant>) {
        self.series[phase as usize].finish(start);
    }

    pub fn report(&mut self) {
        if !self.series[0].enabled {
            return;
        }
        if let Some(c) = &mut self.cadence { c.report(); }
        self.ppu.report();
        if self.gpu.enabled {
            let c = self.gpu_counts;
            println!("FFTA_GPU_COUNTS submitted={} completed={} failed={} invalid={} poll_errors={} busy_skipped={} limit_skipped={} pending={}", c.submitted, c.completed, c.failed, c.invalid, c.poll_errors, c.busy_skipped, c.limit_skipped, c.pending);
            self.gpu.report_named("FFTA_GPU_PROFILE phase=render_pass");
        }
        println!(
            "FFTA_UI_UPLOAD samples={} payload_bytes={}",
            self.ui_updates, self.ui_payload_bytes
        );
        for (series, name) in self.series.iter_mut().zip(PHASE_NAMES) {
            series.report_named(&format!("FFTA_PLAY_PROFILE phase={name}"));
        }
    }
}

struct PresentationCadence {
    window: Option<crate::profile_window::FrameWindow>,
    intervals: FrameTimes,
    last: Option<(Instant, u64, bool)>,
    frame: u64,
    active: bool,
    presentations: u64,
    anchors: u64,
    skipped: u64,
    excluded: u64,
    renderer_changes: u64,
    repeated_guest: u64,
    multi_guest: u64,
    guest_frames: u64,
}

impl PresentationCadence {
    fn new(window: Option<&str>, limit: usize) -> Result<Self, String> {
        Ok(Self {
            window: window.map(|v| crate::profile_window::FrameWindow::parse(v, "FFTA_PRESENT_PROFILE_FRAMES")).transpose()?,
            intervals: FrameTimes::with_limit(true, limit), last: None, frame: 0, active: false,
            presentations: 0, anchors: 0, skipped: 0, excluded: 0, renderer_changes: 0,
            repeated_guest: 0, multi_guest: 0, guest_frames: 0,
        })
    }

    fn prepare(&mut self, frame: u64, eligible: bool) {
        if let Some(w) = &mut self.window { w.publish(frame); }
        let active = eligible && self.window.as_ref().is_none_or(|w| w.active());
        if !active || !self.active || frame < self.frame { self.last = None; }
        self.frame = frame;
        self.active = active;
        if !active { self.excluded += 1; }
    }

    fn presented(&mut self, now: Instant, software: bool) {
        if !self.active { return; }
        self.presentations += 1;
        if let Some((previous, frame, was_software)) = self.last {
            if software == was_software {
                self.intervals.record(now.saturating_duration_since(previous));
                let delta = self.frame.saturating_sub(frame);
                self.repeated_guest += u64::from(delta == 0);
                self.multi_guest += u64::from(delta > 1);
                self.guest_frames += delta;
            } else {
                self.renderer_changes += 1;
                self.anchors += 1;
            }
        } else { self.anchors += 1; }
        self.last = Some((now, self.frame, software));
    }

    fn report(&mut self) {
        if let Some(w) = &self.window {
            println!("FFTA_PRESENT_WINDOW start={} end_exclusive={} last_published={} closed={} interrupted={}",
                w.start, w.end, w.frame.map_or_else(|| "none".into(), |f| f.to_string()), w.closed, w.interrupted);
        }
        println!("FFTA_PRESENT_COUNTS presentations={} anchors={} skipped={} excluded_attempts={} renderer_changes={} repeated_guest={} multi_guest={} guest_frames={}",
            self.presentations, self.anchors, self.skipped, self.excluded, self.renderer_changes,
            self.repeated_guest, self.multi_guest, self.guest_frames);
        self.intervals.report_named("FFTA_PRESENT_INTERVAL");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_intervals_keep_surface_skips_but_cut_ineligible_epochs() {
        let mut p = PlayProfile::new(true);
        p.cadence = Some(PresentationCadence::new(None, 8).unwrap());
        let t = Instant::now();
        p.prepare_presentation(10, true); p.presented(t, false);
        p.prepare_presentation(11, true); p.presentation_skipped();
        p.prepare_presentation(12, true); p.presented(t + Duration::from_millis(34), false);
        let c = p.cadence.as_ref().unwrap();
        assert_eq!(c.intervals.samples, [Duration::from_millis(34)]);
        assert_eq!((c.multi_guest, c.guest_frames, c.skipped), (1, 2, 1));
        p.prepare_presentation(12, false); // pause, priming, fast-forward or zero-size
        p.prepare_presentation(13, true); p.presented(t + Duration::from_secs(10), false);
        assert_eq!(p.cadence.as_ref().unwrap().intervals.samples.len(), 1);
        p.prepare_presentation(13, true); p.presented(t + Duration::from_millis(10_017), false);
        assert_eq!(p.cadence.as_ref().unwrap().repeated_guest, 1);
        p.prepare_presentation(14, true); p.presented(t + Duration::from_millis(10_034), true);
        let c = p.cadence.as_ref().unwrap();
        assert_eq!((c.renderer_changes, c.anchors), (1, 3));
        assert_eq!(c.presentations, c.anchors + c.intervals.samples.len() as u64);
    }

    #[test]
    fn presentation_window_caps_samples_and_does_not_reenter_after_restore() {
        let mut p = PlayProfile::new(true);
        p.cadence = Some(PresentationCadence::new(Some("100:104"), 1).unwrap());
        let t = Instant::now();
        for frame in 99..=105 {
            p.prepare_presentation(frame, true);
            p.presented(t + Duration::from_millis(frame), false);
        }
        let c = p.cadence.as_ref().unwrap();
        assert_eq!((c.presentations, c.anchors, c.intervals.samples.len(), c.intervals.omitted), (4, 1, 1, 2));
        assert!(c.window.as_ref().unwrap().closed);
        p.presentation_restored(101); p.prepare_presentation(101, true); p.presented(t, false);
        assert_eq!(p.cadence.as_ref().unwrap().presentations, 4);
        for restored in [99, 100, 102, 200] {
            let mut p = PlayProfile::new(true);
            p.configure_cadence(Some("100:104")).unwrap();
            p.prepare_presentation(100, true); p.presented(t, false);
            p.presentation_restored(restored);
            p.prepare_presentation(101, true); p.presented(t, false);
            let c = p.cadence.as_ref().unwrap();
            assert_eq!(c.presentations, 1);
            assert!(c.window.as_ref().unwrap().interrupted);
        }
    }

    #[test]
    fn cadence_is_optional_and_configuration_boundaries_start_new_anchors() {
        let mut off = PlayProfile::new(false);
        off.configure_cadence(Some("invalid")).unwrap();
        off.prepare_presentation(10, true); off.presented(Instant::now(), false);
        assert!(off.cadence.is_none());
        let mut benchmark = PlayProfile::new(true);
        benchmark.prepare_presentation(10, true); benchmark.presented(Instant::now(), false);
        assert!(benchmark.cadence.is_none()); // enabled only by normal-play setup
        assert!(PresentationCadence::new(Some("10:10"), 1).is_err());
        let mut p = PlayProfile::new(true); p.configure_cadence(None).unwrap();
        let t = Instant::now();
        p.prepare_presentation(10, true); p.presented(t, false);
        p.presentation_boundary();
        p.prepare_presentation(11, true); p.presented(t + Duration::from_secs(1), false);
        let c = p.cadence.as_ref().unwrap();
        assert_eq!((c.presentations, c.anchors, c.intervals.samples.len()), (2, 2, 0));
    }

    #[test]
    fn ppu_profile_records_disjoint_frame_totals_and_bounds_storage() {
        let mut profile = PpuProfile::with_limit(true, 1);
        let timings = gba_core::ppu_profile::Timings {
            frame: Duration::from_micros(10),
            scanlines: Duration::from_micros(6),
            zoom: Duration::from_micros(3),
            scanline_calls: 160,
            zoom_calls: 1,
            ..Default::default()
        };
        profile.record(Some(timings));
        profile.record(Some(timings));
        assert_eq!(
            (
                profile.frames,
                profile.scanlines,
                profile.zoom,
                profile.invalid
            ),
            (2, 320, 2, 0)
        );
        for (series, micros) in profile.series.iter().zip([6, 3, 1]) {
            assert_eq!(series.samples, [Duration::from_micros(micros)]);
            assert_eq!(series.omitted, 1);
        }
        profile.record(Some(gba_core::ppu_profile::Timings {
            frame: Duration::ZERO,
            ..timings
        }));
        assert_eq!(profile.invalid, 1);
        let mut disabled = PpuProfile::new(false);
        disabled.record(Some(timings));
        assert_eq!(disabled.frames, 0);
        assert!(disabled.series.iter().all(|s| s.samples.capacity() == 0));
    }

    #[test]
    fn detailed_stages_partition_scanlines_and_detect_overcounting() {
        let mut profile = PpuProfile::with_detail_limit(true, true, 1);
        let timings = gba_core::ppu_profile::Timings {
            frame: Duration::from_micros(20),
            scanlines: Duration::from_micros(12),
            detailed: true,
            stages: [Duration::from_micros(2); 5],
            ..Default::default()
        };
        profile.record(Some(timings));
        assert_eq!(profile.invalid, 0);
        assert_eq!(
            profile
                .stages
                .iter()
                .map(|s| s.samples[0])
                .sum::<Duration>(),
            timings.scanlines
        );
        profile.record(Some(timings));
        assert!(profile
            .stages
            .iter()
            .all(|s| s.samples.len() == 1 && s.omitted == 1));
        profile.record(Some(gba_core::ppu_profile::Timings {
            scanlines: Duration::ZERO,
            ..timings
        }));
        assert_eq!(profile.invalid, 1);
        let disabled = PpuProfile::with_detail_limit(false, true, 1);
        assert!(!disabled.detailed());
        assert!(disabled.stages.iter().all(|s| s.samples.capacity() == 0));
    }

    #[test]
    fn disabled_and_warmup_never_start_a_clock() {
        let mut off = FrameTimes::new(false);
        assert!(off.start(true).is_none());
        off.finish(None);
        assert!(off.samples.is_empty());
        assert!(FrameTimes::new(true).start(false).is_none());
    }

    #[test]
    fn capture_is_bounded_and_counts_omitted_samples() {
        let mut times = FrameTimes::new(true);
        for _ in 0..MAX_SAMPLES + 3 {
            times.record(Duration::from_millis(2));
        }
        assert_eq!(times.samples.len(), MAX_SAMPLES);
        assert_eq!(times.omitted, 3);
    }
    #[test]
    fn delayed_gpu_batches_keep_pending_as_a_gauge_and_ignore_disabled_capture() {
        let mut profile = PlayProfile::new(false);
        profile.gpu = FrameTimes::with_limit(true, 2);
        let batch = || screen::present::GpuTimings {
            samples: [Some(Duration::from_micros(20)), None, None],
            counts: screen::present::GpuTimingCounts {
                submitted: 1, completed: 1, pending: 2, ..Default::default()
            },
        };
        profile.record_gpu(batch());
        profile.record_gpu(batch());
        assert_eq!(profile.gpu.samples.len(), 2);
        assert_eq!((profile.gpu_counts.submitted, profile.gpu_counts.completed, profile.gpu_counts.pending), (2, 2, 2));
        let mut disabled = PlayProfile::new(false);
        disabled.record_gpu(batch());
        assert!(disabled.gpu.samples.is_empty());
        assert_eq!(disabled.gpu_counts.completed, 0);
    }

    #[test]
    fn measurement_gate_excludes_warmup_without_disabling_requested_gpu_features() {
        let mut profile = PlayProfile::new(true);
        profile.gpu = FrameTimes::with_limit(true, 1);
        profile.set_measuring(false);
        assert!(!profile.enabled());
        assert!(profile.start().is_none());
        assert!(profile.gpu_enabled());
        profile.record(Phase::Core, Some(Duration::from_millis(1)));
        profile.record_ui_upload(4);
        assert!(profile.series[0].samples.is_empty());
        assert_eq!(profile.ui_updates, 0);
        profile.set_measuring(true);
        assert!(profile.start().is_some());
        profile.record(Phase::Core, Some(Duration::from_millis(1)));
        assert_eq!(profile.series[0].samples.len(), 1);
    }

}
