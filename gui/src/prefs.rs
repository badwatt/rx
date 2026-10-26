//! Preferences that survive a restart.
//!
//! Only the GUI persists anything: the CLI keeps its documented defaults and
//! stays flag-driven.  The file is a tiny `key=value` text format, so the app
//! needs no serialization dependency.

use std::path::{Path, PathBuf};

/// The three toggles of the confirmation sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Prefs {
    pub(crate) update: bool,
    pub(crate) diff: bool,
    pub(crate) dry_run: bool,
}

impl Prefs {
    /// Read the preferences, falling back to the defaults when absent.
    pub(crate) fn load() -> Self {
        path().map_or_else(Self::default, |path| Self::load_from(&path))
    }

    /// Persist the preferences.  Failures are silent: a read-only home must
    /// not stop an update.
    pub(crate) fn save(self) {
        if let Some(path) = path() {
            self.save_to(&path);
        }
    }

    fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path).map_or_else(|_| Self::default(), |text| parse(&text))
    }

    fn save_to(self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, render(self));
    }
}

/// `$XDG_CONFIG_HOME/rxui/config`, or `~/.config/rxui/config`.
fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("rxui").join("config"))
}

/// Parse `key=value` lines; unknown keys and junk are ignored.
fn parse(text: &str) -> Prefs {
    let mut prefs = Prefs::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let flag = value.trim() == "true";
        match key.trim() {
            "update" => prefs.update = flag,
            "diff" => prefs.diff = flag,
            "dry_run" => prefs.dry_run = flag,
            _ => {}
        }
    }
    prefs
}

fn render(prefs: Prefs) -> String {
    format!(
        "update={}\ndiff={}\ndry_run={}\n",
        prefs.update, prefs.diff, prefs.dry_run
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_every_known_key() {
        let prefs = parse("update=true\ndiff=true\ndry_run=true\n");
        assert_eq!(
            prefs,
            Prefs {
                update: true,
                diff: true,
                dry_run: true
            }
        );
    }

    #[test]
    fn parse_ignores_unknown_and_malformed_lines() {
        let prefs = parse("noise\nupdate=true\nfuture=1\ndiff\n");
        assert_eq!(
            prefs,
            Prefs {
                update: true,
                ..Prefs::default()
            }
        );
    }

    #[test]
    fn render_round_trips_through_parse() {
        let prefs = Prefs {
            update: true,
            diff: false,
            dry_run: true,
        };
        assert_eq!(parse(&render(prefs)), prefs);
    }

    #[test]
    fn load_from_missing_file_yields_defaults() {
        let missing = std::env::temp_dir().join("rxui-no-such-config");
        assert_eq!(Prefs::load_from(&missing), Prefs::default());
    }

    #[test]
    fn save_to_then_load_from_keeps_the_toggles() {
        let dir = std::env::temp_dir().join(format!("rxui-prefs-{}", std::process::id()));
        let path = dir.join("nested").join("config");
        let prefs = Prefs {
            update: true,
            diff: true,
            dry_run: false,
        };
        prefs.save_to(&path);
        assert_eq!(Prefs::load_from(&path), prefs);
        let _ = std::fs::remove_dir_all(dir);
    }
}
