//! Host-only Fast consumer clock. Production is measured in guest PCM pairs;
//! elapsed time is device frames, so this needs no wall-clock calls or locks.

pub struct Clock {
    source_rate: f64,
    output_rate: f64,
    produced: u64,
    estimate: f64,
    speed: f64,
    target: f64,
    slew: f64,
    ready: bool,
}

#[derive(Clone, Copy)]
pub struct Plan {
    pub ready: bool,
    pub rebuffered: bool,
    pub estimate: f64,
    pub speed: f64,
}

impl Clock {
    pub fn new(source_rate: f64, output_rate: f64) -> Self {
        Self {
            source_rate, output_rate, produced: 0, estimate: 1., speed: 1.,
            target: 1., slew: 1. / (0.100 * output_rate), ready: false,
        }
    }

    pub fn reset(&mut self) {
        self.produced = 0;
        self.estimate = 1.;
        self.speed = 1.;
        self.target = 1.;
        self.ready = false;
    }

    pub fn begin(&mut self, produced: u64, queued: usize, frames: usize, requested: f64) -> Plan {
        let was_ready = self.ready;
        if frames == 0 {
            return Plan { ready: self.ready, rebuffered: false, estimate: self.estimate, speed: self.speed };
        }
        let dt = frames as f64 / self.output_rate;
        // The same 2% clock tolerance as ordinary playback also handles small
        // producer/device frequency differences at the requested upper speed.
        let limit = requested.clamp(1., 8.) * 1.02;
        let incoming = produced.wrapping_sub(self.produced) as f64 / self.source_rate / dt;
        self.produced = produced;
        // ~200 ms production filter rejects individual guest-frame bursts.
        self.estimate += (incoming - self.estimate) * (dt / (0.200 + dt));
        self.estimate = self.estimate.clamp(0.05, limit);
        // Keep 60 ms after this callback, plus its own complete source demand.
        // Queue feedback removes estimator bias without chasing each arrival.
        let reserve = self.estimate * (0.060 + dt);
        let queued_seconds = queued as f64 / self.source_rate;
        self.target = (self.estimate + (queued_seconds - reserve) / 0.250).clamp(0.05, limit);
        if !self.ready {
            self.ready = queued_seconds >= reserve;
            if self.ready { self.speed = self.estimate; }
        }
        // A discontinuity can exhaust even the reserve. Re-prime with silence,
        // rather than stretching a stale DAC level through repeated underruns.
        // Two pairs cover fractional phase rounding in all grid consumers.
        if self.ready && (queued as f64) < self.source_rate * dt * self.speed.max(self.target) + 2. {
            self.ready = false;
        }
        Plan {
            ready: self.ready, rebuffered: was_ready && !self.ready,
            estimate: self.estimate, speed: self.speed,
        }
    }

    pub fn next(&mut self) -> f64 {
        // Shared by every source for this device sample. No independent queue
        // steering in Fast; fractional sample rate changes continuously.
        self.speed += (self.target - self.speed) * self.slew;
        self.speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_production_tracks_slow_and_full_rates_with_bounded_reserve() {
        for rate in [44_100., 48_000., 96_000.] {
            for frames in [256, 2048] {
                for requested in [2., 4., 8.] {
                    for actual in [0.5, 1., 1.75, requested] {
                        let mut clock = Clock::new(65_536., rate);
                        let mut produced = 0.;
                        let mut fed = 0u64;
                        let mut queue = 0.;
                        let mut silent = 0;
                        let callbacks = (8. * rate / frames as f64) as usize;
                        for i in 0..callbacks {
                            produced += 65_536. * actual * frames as f64 / rate;
                            let incoming = ((produced as u64 - fed) / 1097) * 1097;
                            fed += incoming;
                            queue += incoming as f64;
                            let plan = clock.begin(fed, queue.floor() as usize, frames, requested);
                            if plan.ready {
                                for _ in 0..frames {
                                    let before = clock.speed;
                                    let speed = clock.next();
                                    assert!((speed - before).abs() < 0.002);
                                    assert!((0.05..=requested * 1.02).contains(&speed));
                                    queue -= 65_536. / rate * speed;
                                }
                                assert!(queue >= 0., "queue drained: {rate}/{frames}/{actual}/{requested}");
                            } else if i > callbacks / 2 { silent += 1; }
                            if i > callbacks / 2 {
                                assert!(queue / 65_536. / actual < 0.20,
                                    "growing latency: {rate}/{frames}/{actual}/{requested}");
                            }
                        }
                        assert_eq!(silent, 0, "steady rebuffer: {rate}/{frames}/{actual}/{requested}");
                        assert!((clock.speed - actual).abs() / actual < 0.05);
                    }
                }
            }
        }
    }

    #[test]
    fn rate_changes_and_production_stalls_recover_without_persistent_silence() {
        let mut clock = Clock::new(65_536., 48_000.);
        let mut produced = 0.;
        let mut fed = 0u64;
        let mut queue = 0.;
        for actual in [4., 1., 3., 0., 2.] {
            let mut last_half_waiting = 0;
            for tick in 0..400 {
                produced += 65_536. * actual * 512. / 48_000.;
                let incoming = ((produced as u64 - fed) / 1097) * 1097;
                fed += incoming;
                queue += incoming as f64;
                let plan = clock.begin(fed, queue.floor() as usize, 512, 4.);
                if plan.ready {
                    for _ in 0..512 { queue -= clock.next() * 65_536. / 48_000.; }
                    assert!(queue >= 0.);
                } else if tick > 200 { last_half_waiting += 1; }
                if actual > 0. && tick > 200 {
                    assert!(queue / 65_536. / actual < 0.20);
                }
            }
            if actual > 0. { assert_eq!(last_half_waiting, 0, "{actual}"); }
            else { assert!(last_half_waiting > 0); }
        }
        assert!(clock.begin(fed, queue.floor() as usize, 0, 4.).ready);
        // Old production history cannot affect a load/multiplier transition.
        clock.reset();
        assert!(!clock.begin(0, 0, 512, 8.).ready);
        assert!(clock.estimate < 1.);
        // Empty device buffers do not consume producer observations.
        let previous = clock.produced;
        clock.begin(500, 500, 0, 8.);
        assert_eq!(clock.produced, previous);
    }
}
