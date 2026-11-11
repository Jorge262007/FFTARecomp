#![cfg_attr(windows, windows_subsystem = "windows")]
use std::{env, fs, io::{BufRead, BufReader, Write}, path::{Path, PathBuf}, process::{Command, Stdio}};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn fail(dir: &PathBuf, msg: &str) {
    let _ = fs::write(dir.join("play-error.log"), msg);
}

/// Options a launcher passes. A plain start has none and behaves as it always has.
struct Options { game: Option<PathBuf>, prepare: bool }

fn parse_options() -> Options {
    let mut options = Options { game: None, prepare: false };
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--prepare" { options.prepare = true; }
        else if arg == "--game" { options.game = args.next().map(PathBuf::from); }
        else if let Some(path) = arg.to_str().and_then(|a| a.strip_prefix("--game=")) { options.game = Some(PathBuf::from(path)); }
    }
    options
}

/// A ROM picked by a launcher lives in the player's own library, so keep the save with the
/// game's data, where the installer puts it, unless the player already chose a place.
fn keep_save_with_game_data(command: &mut Command, data_dir: &Path) {
    if env::var_os("FFTA_SAVE_PATH").is_some() { return; }
    if data_dir.join("shared-save-path.txt").is_file() { return; }
    command.env("FFTA_SAVE_PATH", data_dir.join("ffta.sav"));
}

/// Turns recomp's `STATUS building <pct> <text>` into the `progress <0-1> <text>` lines a launcher reads.
fn progress_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix("STATUS building ")?;
    let (pct, text) = rest.split_once(' ').unwrap_or((rest, ""));
    let fraction = pct.parse::<f32>().ok()? / 100.0;
    let text = if text.is_empty() { "Translating the game" } else { text };
    Some(format!("progress {fraction:.2} {text}"))
}

fn report(line: &str) {
    // Ignore a closed pipe: a missing reader must not turn into a failed setup.
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// Runs the one-time translation without a window and exits with recomp's own exit code.
fn prepare(mut command: Command, exe_dir: &PathBuf) {
    let log = fs::File::create(exe_dir.join("prepare.log")).map(Stdio::from).unwrap_or_else(|_| Stdio::null());
    command.stdout(Stdio::piped()).stderr(log);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            fail(exe_dir, &format!("No se pudo lanzar recomp: {e}"));
            std::process::exit(1);
        }
    };
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(progress) = progress_line(&line) { report(&progress); }
        }
    }
    let code = child.wait().ok().and_then(|status| status.code()).unwrap_or(1);
    if code == 0 { report("progress 1 Ready"); }
    std::process::exit(code);
}

fn main() {
    let options = parse_options();
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let recomp = exe_dir.join(if cfg!(windows){"recomp.exe"}else{"recomp"});
    let toolchain = exe_dir.join("toolchain").join("bin");

    let data_dir = dirs::data_local_dir().unwrap_or_else(||PathBuf::from(".")).join("FFTARecomp");
    let local = data_dir.join("ffta.gba");
    let rom = match options.game.clone() {
        Some(game) => game,
        None if exe_dir.join("ffta.gba").exists() => exe_dir.join("ffta.gba"),
        None => local,
    };

    if !recomp.exists() { return fail(&exe_dir, &format!("Falta {}", recomp.display())); }
    if !rom.exists()    { return fail(&exe_dir, &format!("Falta la ROM: {}", rom.display())); }

    let mut command=Command::new(&recomp);
    command.arg("play");
    if options.prepare { command.arg("--prepare").arg("--status"); }
    if exe_dir.join("gamedb.sqlite").is_file(){command.arg("--gamedb").arg(exe_dir.join("gamedb.sqlite"));}
    command.arg(&rom).current_dir(&exe_dir).env("RECOMP_JOBS","2");
    #[cfg(any(target_os = "linux", windows))]
    command.env("FFTA_FRAME_PACING", env::var_os("FFTA_FRAME_PACING").unwrap_or_else(|| "clock".into()));
    let old=env::var_os("PATH").unwrap_or_default();let mut paths=vec![toolchain];
    paths.extend(env::split_paths(&old));if let Ok(path)=env::join_paths(paths){command.env("PATH",path);}
    if options.game.is_some() { keep_save_with_game_data(&mut command, &data_dir); }
    #[cfg(windows)] command.creation_flags(0x0800_0000);
    if options.prepare { return prepare(command, &exe_dir); }
    let r=command.output();

    #[cfg(target_os = "linux")]
    if let Ok(output) = &r {
        let log = format!(
            "recomp exit: {}\n--- stderr ---\n{}\n--- stdout ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        let _ = fs::write(exe_dir.join("play.log"), log);
    }

    match r {
        Err(e) => fail(&exe_dir, &format!("No se pudo lanzar recomp.exe: {e}")),
        Ok(o) if !o.status.success() => fail(
            &exe_dir,
            &format!(
                "recomp.exe termino con {}\n--- stderr ---\n{}\n--- stdout ---\n{}",
                o.status,
                String::from_utf8_lossy(&o.stderr),
                String::from_utf8_lossy(&o.stdout)
            ),
        ),
        Ok(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::progress_line;

    #[test]
    fn building_status_becomes_a_progress_line() {
        assert_eq!(progress_line("STATUS building 40 Compiling chunk 2/5").as_deref(), Some("progress 0.40 Compiling chunk 2/5"));
    }

    #[test]
    fn building_status_without_text_gets_a_default() {
        assert_eq!(progress_line("STATUS building 0").as_deref(), Some("progress 0.00 Translating the game"));
    }

    #[test]
    fn other_lines_are_dropped() {
        assert_eq!(progress_line("STATUS playing"), None);
        assert_eq!(progress_line("native translation: 1200 blocks"), None);
    }
}
