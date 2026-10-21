#![cfg_attr(windows, windows_subsystem = "windows")]
use std::{env, fs, path::PathBuf, process::Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn fail(dir: &PathBuf, msg: &str) {
    let _ = fs::write(dir.join("play-error.log"), msg);
    rfd::MessageDialog::new().set_title("FFTARecomp")
        .set_description(msg).set_level(rfd::MessageLevel::Error).show();
}

fn main() {
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let recomp = exe_dir.join(if cfg!(windows){"recomp.exe"}else{"recomp"});
    let toolchain = exe_dir.join("toolchain").join("bin");

    let local = dirs::data_local_dir().unwrap_or_else(||PathBuf::from("."))
        .join("FFTARecomp").join("ffta.gba");
    let portable = input_config::storage::portable_root().is_some();
    let bundled_rom = exe_dir.join("ffta.gba");
    let rom = if bundled_rom.is_file() || portable { bundled_rom } else { local };

    if !recomp.exists() { return fail(&exe_dir, &format!("Falta {}", recomp.display())); }
    if !rom.is_file() && portable {
        let Some(selected) = rfd::FileDialog::new()
            .set_title("Choose your FFTA ROM (copied into the portable folder)")
            .add_filter("Game Boy Advance ROM", &["gba"]).pick_file() else { return; };
        // Never overwrite a file that appeared while the picker was open.
        let result = (|| -> std::io::Result<()> {
            use std::io::Write;
            let bytes = fs::read(&selected)?;
            let mut file = tempfile::NamedTempFile::new_in(&exe_dir)?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            file.persist_noclobber(&rom).map_err(|e| e.error)?;
            Ok(())
        })();
        if let Err(e) = result { return fail(&exe_dir, &format!("Cannot copy ROM: {e}")); }
    }
    if !rom.is_file() { return fail(&exe_dir, &format!("Missing ROM: {}", rom.display())); }

    let mut command=Command::new(&recomp);
    command.arg("play");
    if exe_dir.join("gamedb.sqlite").is_file(){command.arg("--gamedb").arg(exe_dir.join("gamedb.sqlite"));}
    command.arg(&rom).current_dir(&exe_dir).env("RECOMP_JOBS","2");
    #[cfg(any(target_os = "linux", windows))]
    command.env("FFTA_FRAME_PACING", env::var_os("FFTA_FRAME_PACING").unwrap_or_else(|| "clock".into()));
    let old=env::var_os("PATH").unwrap_or_default();let mut paths=vec![toolchain];
    paths.extend(env::split_paths(&old));if let Ok(path)=env::join_paths(paths){command.env("PATH",path);}
    #[cfg(windows)] command.creation_flags(0x0800_0000);
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
