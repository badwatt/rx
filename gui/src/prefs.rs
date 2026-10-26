//! Preferences that survive a restart.
//!
//! Only the GUI writes them: the CLI keeps its documented defaults and stays
//! flag-driven.  They live in the shared `rx/config.toml`, under a `[ui]`
//! table, so the CLI can grow its own sections without a second file.

use std::path::{Path, PathBuf};

/// Section owned by the desktop surface.
const SECTION: &str = "ui";

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
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        let _ = std::fs::write(path, merge(&existing, self));
    }
}

/// `$XDG_CONFIG_HOME/rx/config.toml`, or `~/.config/rx/config.toml`.
fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("rx").join("config.toml"))
}

fn is_section(line: &str, section: &str) -> bool {
    line.trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .is_some_and(|name| name.trim() == section)
}

/// Parse the `[ui]` table; other sections and junk are ignored.
fn parse(text: &str) -> Prefs {
    let mut prefs = Prefs::default();
    let mut in_ui = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_ui = is_section(line, SECTION);
            continue;
        }
        if !in_ui {
            continue;
        }
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

fn render_block(prefs: Prefs) -> String {
    format!(
        "[{SECTION}]\nupdate = {}\ndiff = {}\ndry_run = {}\n",
        prefs.update, prefs.diff, prefs.dry_run
    )
}

/// Replace the `[ui]` table in place, keeping every other section.
fn merge(existing: &str, prefs: Prefs) -> String {
    let mut kept = String::new();
    let mut in_ui = false;
    for line in existing.lines() {
        if line.trim_start().starts_with('[') {
            in_ui = is_section(line, SECTION);
            if in_ui {
                continue;
            }
        }
        if !in_ui {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    let kept = kept.trim_end();
    if kept.is_empty() {
        format!("# rx configuration\n\n{}", render_block(prefs))
    } else {
        format!("{kept}\n\n{}", render_block(prefs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_the_ui_table() {
        let prefs = parse("[ui]\nupdate = true\ndiff = true\ndry_run = true\n");
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
    fn parse_ignores_other_sections_and_junk() {
        let prefs = parse("[cli]\nupdate = true\n\n[ui]\ndiff = true\nnoise\n");
        assert_eq!(
            prefs,
            Prefs {
                diff: true,
                ..Prefs::default()
            }
        );
    }

    #[test]
    fn merge_keeps_foreign_sections_and_replaces_the_ui_table() {
        let existing = "[cli]\nrepo = \"/tmp/dotfiles\"\n\n[ui]\nupdate = true\n";
        let prefs = Prefs {
            update: false,
            diff: true,
            dry_run: false,
        };
        let merged = merge(existing, prefs);
        assert!(merged.contains("repo = \"/tmp/dotfiles\""));
        assert!(!merged.contains("update = true"));
        assert_eq!(parse(&merged), prefs);
    }

    #[test]
    fn render_round_trips_through_parse() {
        let prefs = Prefs {
            update: true,
            diff: false,
            dry_run: true,
        };
        assert_eq!(parse(&merge("", prefs)), prefs);
    }

    #[test]
    fn load_from_missing_file_yields_defaults() {
        let missing = std::env::temp_dir().join("rxui-no-such-config.toml");
        assert_eq!(Prefs::load_from(&missing), Prefs::default());
    }

    #[test]
    fn save_to_then_load_from_keeps_the_toggles() {
        let dir = std::env::temp_dir().join(format!("rxui-prefs-{}", std::process::id()));
        let path = dir.join("nested").join("config.toml");
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
