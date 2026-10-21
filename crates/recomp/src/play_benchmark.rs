//! Developer-only, bounded GUI workload. Reuses native frame/presentation code,
//! but runs one replay frame per cycle without audio callbacks or clock pacing.
use crate::InputScript;
use gba_core::Machine;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(super) struct Benchmark {
    state: PathBuf,
    input: InputScript,
    pub frames: u64,
    pub warmup: u64,
    pub zoom: u32,
    scene: String,
    verify: bool,
    first: u64,
    completed: u64,
    total: Duration,
    samples: Vec<Duration>,
}

impl Benchmark {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text, path.parent().unwrap_or(Path::new(".")))
    }

    fn parse(text: &str, directory: &Path) -> Result<Self, String> {
        let table = text
            .parse::<toml::Table>()
            .map_err(|e| format!("benchmark config: {e}"))?;
        for key in table.keys() {
            if ![
                "state", "input", "frames", "warmup", "zoom", "scene", "verify",
            ]
            .contains(&key.as_str())
            {
                return Err(format!("unknown benchmark setting {key:?}"));
            }
        }
        let number = |key: &str, default: Option<u64>| -> Result<u64, String> {
            match table.get(key) {
                None => default.ok_or_else(|| format!("benchmark needs {key}")),
                Some(value) => value
                    .as_integer()
                    .and_then(|n| u64::try_from(n).ok())
                    .ok_or_else(|| format!("benchmark {key} must be a nonnegative integer")),
            }
        };
        let file = |key: &str| -> Result<PathBuf, String> {
            let name = table
                .get(key)
                .and_then(toml::Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("benchmark needs {key} path"))?;
            Ok(directory.join(name))
        };
        let frames = number("frames", None)?;
        let warmup = number("warmup", Some(120))?;
        if frames == 0 || frames > 360_000 || warmup >= frames {
            return Err("benchmark needs 0 <= warmup < frames <= 360000".into());
        }
        let zoom =
            u32::try_from(number("zoom", Some(0))?).map_err(|_| "benchmark zoom is too large")?;
        let scene = table
            .get("scene")
            .map(|v| v.as_str().ok_or("benchmark scene must be a string"))
            .transpose()?
            .unwrap_or("replay")
            .to_owned();
        if scene.is_empty()
            || scene.len() > 64
            || !scene
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("benchmark scene must be 1-64 ASCII letters, digits, _ or -".into());
        }
        let verify = table
            .get("verify")
            .map(|v| v.as_bool().ok_or("benchmark verify must be boolean"))
            .transpose()?
            .unwrap_or(false);
        let input_path = file("input")?;
        let input = InputScript::load(
            input_path
                .to_str()
                .ok_or("benchmark input path is not UTF-8")?,
        )?;
        Ok(Self {
            state: file("state")?,
            input,
            frames,
            warmup,
            zoom,
            scene,
            verify,
            first: 0,
            completed: 0,
            total: Duration::ZERO,
            samples: Vec::with_capacity((frames - warmup) as usize),
        })
    }

    pub fn restore(&mut self, machine: &mut Machine) -> Result<(), String> {
        crate::save_state::load(machine, &self.state)?;
        self.first = machine.bus.frames;
        self.first
            .checked_add(self.frames)
            .ok_or("benchmark frame range overflow")?;
        println!("FFTA_GUI_BENCH_START scene={} first={} frames={} warmup={} zoom={} verify={} mode=unpaced_audio_discard initial_state={}",
            self.scene, self.first, self.frames, self.warmup, self.zoom, self.verify, gba_core::ffta_rom::digest(&machine.save_state()?));
        Ok(())
    }

    pub fn measuring(&self) -> bool {
        self.completed >= self.warmup
    }

    pub fn apply_input(&self, machine: &mut Machine) -> Result<(), String> {
        if machine.bus.frames != self.first + self.completed {
            return Err("benchmark guest frame drift".into());
        }
        machine.bus.keys = self.input.keys_at(self.completed);
        self.input.apply_mouse(&mut machine.bus, self.completed)
    }

    /// End timing before hashing/logging. Verification runs are separate from timing.
    pub fn finish_frame(&mut self, machine: &Machine, start: Instant) -> Result<bool, String> {
        let elapsed = start.elapsed();
        if machine.bus.frames != self.first + self.completed + 1 {
            return Err("benchmark incomplete or skipped guest frame".into());
        }
        if self.measuring() {
            self.samples.push(elapsed);
            self.total += elapsed;
        }
        self.completed += 1;
        if self.verify {
            println!(
                "FFTA_GUI_VERIFY frame={} framebuffer={:016x} oam={:016x}",
                self.completed,
                crate::fb_hash(machine),
                super::fnv64(&machine.bus.oam)
            );
        }
        Ok(self.completed == self.frames)
    }

    pub fn report(&mut self, machine: &Machine, native: u64, fallback: u64) -> Result<(), String> {
        if self.completed != self.frames {
            return Err(format!(
                "benchmark interrupted at {}/{} frames",
                self.completed, self.frames
            ));
        }
        self.samples.sort_unstable();
        let n = self.samples.len();
        let percentile =
            |p: usize| self.samples[(n * p).div_ceil(100).saturating_sub(1)].as_secs_f64() * 1000.;
        println!("FFTA_GUI_BENCH scene={} first={} last={} frames={} warmup={} samples={} p50_ms={:.6} p95_ms={:.6} p99_ms={:.6} max_ms={:.6} mean_ms={:.6} native={} fallback={} final_state={}",
            self.scene, self.first, machine.bus.frames, self.completed, self.warmup, n,
            percentile(50), percentile(95), percentile(99), percentile(100), self.total.as_secs_f64()*1000./n as f64,
            native, fallback, gba_core::ffta_rom::digest(&machine.save_state()?));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_rejects_unbounded_or_ambiguous_runs_and_resolves_relative_inputs() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("input.input"),
            "gba-input v1\n0 3ff\n",
        )
        .unwrap();
        let valid =
            "state='state.ffstate'\ninput='input.input'\nframes=5\nwarmup=2\nscene='battle-1'\n";
        let bench = Benchmark::parse(valid, directory.path()).unwrap();
        assert_eq!(bench.state, directory.path().join("state.ffstate"));
        assert_eq!(bench.samples.capacity(), 3);
        for extra in ["unknown=1", "verify=1", "zoom=-1"] {
            assert!(Benchmark::parse(&format!("{valid}{extra}"), directory.path()).is_err());
        }
        for (a, b) in [
            ("frames=5", "frames=0"),
            ("frames=5", "frames=360001"),
            ("warmup=2", "warmup=5"),
            ("scene='battle-1'", "scene='two scenes'"),
        ] {
            assert!(Benchmark::parse(&valid.replace(a, b), directory.path()).is_err());
        }
    }
    #[test]
    fn warmup_and_frame_guards_do_not_advance_or_mutate_the_machine() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("input.input"),
            "gba-input v1\n0 3fe\n2 3ff\n",
        )
        .unwrap();
        let mut bench = Benchmark::parse(
            "state='s.ffstate'\ninput='input.input'\nframes=3\nwarmup=1",
            directory.path(),
        )
        .unwrap();
        let mut machine = Machine::new(vec![]);
        bench.first = 20;
        machine.bus.frames = 20;
        bench.apply_input(&mut machine).unwrap();
        assert_eq!(machine.bus.keys, 0x3fe);
        assert!(!bench.measuring());
        assert!(bench.finish_frame(&machine, Instant::now()).is_err());
        for i in 1..=3 {
            machine.bus.frames = 20 + i;
            let before = machine.save_state().unwrap();
            assert_eq!(
                bench.finish_frame(&machine, Instant::now()).unwrap(),
                i == 3
            );
            assert!(machine.save_state().unwrap() == before);
        }
        assert_eq!(bench.samples.len(), 2);
        machine.bus.frames += 1;
        assert!(bench.apply_input(&mut machine).is_err());
    }
}
