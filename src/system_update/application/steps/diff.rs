use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Show the closure diff between the previous and current generation.
pub(crate) fn run(nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nix Diff")?;
    let output = nix.diff_closures()?;
    ui.output(&tail_from_last_version(&output))?;
    Ok(())
}

/// Return the tail of `output`, starting at the last line containing `Version`.
///
/// Mirrors the former `tac | awk '/Version/{print; exit} 1' | tac` pipeline:
/// when no `Version` line exists, the whole output is returned unchanged.
pub(crate) fn tail_from_last_version(output: &str) -> String {
    match output.rfind("Version") {
        Some(index) => {
            let start = output[..index].rfind('\n').map_or(0, |newline| newline + 1);
            let mut tail = String::with_capacity(output.len() - start + 1);
            tail.push_str(&output[start..]);
            if !tail.ends_with('\n') {
                tail.push('\n');
            }
            tail
        }
        None => output.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_from_last_version_keeps_from_last_match() {
        let output = "old Version 1\nnoise\nVersion 2\nline a\nline b\n";
        assert_eq!(
            tail_from_last_version(output),
            "Version 2\nline a\nline b\n"
        );
    }

    #[test]
    fn tail_from_last_version_without_match_returns_all() {
        assert_eq!(
            tail_from_last_version("line a\nline b\n"),
            "line a\nline b\n"
        );
    }

    #[test]
    fn tail_from_last_version_empty_returns_empty() {
        assert_eq!(tail_from_last_version(""), "");
    }

    #[test]
    fn tail_from_last_version_match_on_last_line() {
        assert_eq!(tail_from_last_version("a\nVersion 3\n"), "Version 3\n");
    }

    #[test]
    fn tail_from_last_version_adds_a_final_newline() {
        assert_eq!(tail_from_last_version("Version 3"), "Version 3\n");
    }
}
