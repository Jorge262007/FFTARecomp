use crate::path_picker::Kind;
use std::path::PathBuf;

pub enum Result {
    Selected(PathBuf),
    Cancelled,
    Unavailable(String),
}

#[cfg(any(windows, target_os = "macos"))]
pub fn pick(kind: Kind) -> Result {
    let dialog = rfd::FileDialog::new();
    let path = match kind {
        Kind::Rom => dialog.add_filter("GBA ROM", &["gba"]).pick_file(),
        Kind::Folder => dialog.pick_folder(),
    };
    path.map(Result::Selected).unwrap_or(Result::Cancelled)
}

#[cfg(target_os = "linux")]
pub fn pick(kind: Kind) -> Result {
    use std::process::Command;
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_lowercase();
    let tools = if desktop.contains("kde") { ["kdialog", "zenity"] } else { ["zenity", "kdialog"] };
    let mut failures = Vec::new();
    for tool in tools {
        let mut cmd = Command::new(tool);
        if tool == "kdialog" {
            cmd.args(["--title", "FFTARecomp"]);
            match kind {
                Kind::Rom => { cmd.arg("--getopenfilename").arg(&home).arg("*.gba *.GBA|GBA ROM"); },
                Kind::Folder => { cmd.arg("--getexistingdirectory").arg(&home); },
            }
        } else {
            cmd.args(["--file-selection", "--title=FFTARecomp"]);
            let mut start = home.as_os_str().to_os_string();
            start.push("/");
            let mut filename = std::ffi::OsString::from("--filename=");
            filename.push(start);
            cmd.arg(filename);
            match kind {
                Kind::Rom => { cmd.arg("--file-filter=GBA ROM | *.gba *.GBA"); },
                Kind::Folder => { cmd.arg("--directory"); },
            }
        }
        match cmd.output() {
            Ok(output) => match decode(output.status.code(), &output.stdout, &output.stderr, kind) {
                Result::Unavailable(error) => failures.push(format!("{tool}: {error}")),
                result => return result,
            },
            Err(error) => failures.push(format!("{tool}: {error}")),
        }
    }
    Result::Unavailable(failures.join("; "))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn pick(_: Kind) -> Result {
    Result::Unavailable("No desktop selector configured on this platform".into())
}

#[cfg(any(target_os = "linux", test))]
fn decode(code: Option<i32>, stdout: &[u8], stderr: &[u8], kind: Kind) -> Result {
    // Both desktop tools use exit 1 for cancellation; Qt/GTK may still print warnings.
    if code == Some(1) && stdout.is_empty() { return Result::Cancelled; }
    if code != Some(0) {
        return Result::Unavailable(format!("exit {code:?}: {}", String::from_utf8_lossy(stderr)));
    }
    let path_bytes = stdout.strip_suffix(b"\n").unwrap_or(stdout);
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(path_bytes.to_vec()))
    };
    #[cfg(not(unix))]
    let path = PathBuf::from(String::from_utf8_lossy(path_bytes).as_ref());
    let valid = match kind { Kind::Rom => path.is_file(), Kind::Folder => path.is_dir() };
    if valid { Result::Selected(path) }
    else { Result::Unavailable("The desktop dialog did not return an existing path".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_does_not_trigger_another_selector() {
        assert!(matches!(decode(Some(1), b"", b"", Kind::Rom), Result::Cancelled));
        assert!(matches!(decode(Some(1), b"", b"desktop warning", Kind::Rom), Result::Cancelled));
        assert!(matches!(decode(Some(127), b"", b"missing library", Kind::Rom), Result::Unavailable(_)));
    }

    #[test]
    fn validates_returned_folder_and_removes_only_output_newline() {
        let path = std::env::current_dir().unwrap();
        let output = format!("{}\n", path.display());
        assert!(matches!(decode(Some(0), output.as_bytes(), b"", Kind::Folder), Result::Selected(p) if p == path));
        assert!(matches!(decode(Some(0), b"", b"", Kind::Rom), Result::Unavailable(_)));
    }
}
