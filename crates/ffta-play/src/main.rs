#![cfg_attr(windows, windows_subsystem = "windows")]
use std::{env, fs, path::{Path, PathBuf}, process::Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn fail(dir: &PathBuf, msg: &str) {
    let _ = fs::write(dir.join("play-error.log"), msg);
}

/// Folder holding the ROM, gamedb and saves. On macOS the player and recomp
/// run from `FFTARecomp.app/Contents/MacOS`; game data sits beside the bundle
/// so the signed bundle is never modified after install.
fn game_dir(bin_dir: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    if bin_dir.ends_with("Contents/MacOS") {
        if let Some(app) = bin_dir.parent().and_then(|c| c.parent()) {
            if app.extension().is_some_and(|e| e == "app") {
                if let Some(dir) = app.parent() { return dir.to_path_buf(); }
            }
        }
    }
    bin_dir.to_path_buf()
}

fn main() {
    let bin_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let exe_dir = game_dir(&bin_dir);
    let recomp = bin_dir.join(if cfg!(windows){"recomp.exe"}else{"recomp"});
    let toolchain = exe_dir.join("toolchain").join("bin");

    let local = dirs::data_local_dir().unwrap_or_else(||PathBuf::from("."))
        .join("FFTARecomp").join("ffta.gba");
    let rom = if exe_dir.join("ffta.gba").exists() { exe_dir.join("ffta.gba") } else { local };

    if !recomp.exists() { return fail(&exe_dir, &format!("Falta {}", recomp.display())); }
    if !rom.exists()    { return fail(&exe_dir, &format!("Falta la ROM: {}", rom.display())); }

    let mut command=Command::new(&recomp);
    command.arg("play");
    if exe_dir.join("gamedb.sqlite").is_file(){command.arg("--gamedb").arg(exe_dir.join("gamedb.sqlite"));}
    command.arg(&rom).current_dir(&exe_dir).env("RECOMP_JOBS","2");
    #[cfg(any(target_os = "linux", windows))]
    command.env("FFTA_FRAME_PACING", env::var_os("FFTA_FRAME_PACING").unwrap_or_else(|| "clock".into()));
    let old=env::var_os("PATH").unwrap_or_default();let mut paths=vec![toolchain];
    paths.extend(env::split_paths(&old));if let Ok(path)=env::join_paths(paths){command.env("PATH",path);}
    #[cfg(windows)] command.creation_flags(0x0800_0000);
    // Replace this process so the game window belongs to the app Finder
    // launched (one Dock icon, the bundle's name and icon). Output goes to
    // play.log, as on Linux.
    #[cfg(target_os = "macos")]
    if let Ok(log) = fs::File::create(exe_dir.join("play.log")) {
        use std::os::unix::process::CommandExt;
        if let Ok(err) = log.try_clone() { command.stderr(err); }
        let e = command.stdout(log).exec();
        return fail(&exe_dir, &format!("No se pudo lanzar {}: {e}", recomp.display()));
    }
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
        Err(e) => fail(&exe_dir, &format!("No se pudo lanzar {}: {e}", recomp.display())),
        Ok(o) if !o.status.success() => fail(
            &exe_dir,
            &format!(
                "{} termino con {}\n--- stderr ---\n{}\n--- stdout ---\n{}",
                recomp.display(),
                o.status,
                String::from_utf8_lossy(&o.stderr),
                String::from_utf8_lossy(&o.stdout)
            ),
        ),
        Ok(_) => {}
    }
}
