use std::fmt;

use crate::domain::error::Error;

/// Default dotfiles repository path (before `~` expansion).
pub(crate) const DEFAULT_REPO: &str = "~/.dotfiles";
/// Default number of generations to keep during cleanup.
pub(crate) const DEFAULT_KEEP: u8 = 10;

pub(crate) use crate::domain::step::Step;

/// Parallelism caps passed to `nixos-rebuild`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BuildOptions {
    /// `--max-jobs` value.
    pub(crate) jobs: Option<u16>,
    /// `--cores` value.
    pub(crate) cores: Option<u16>,
}

/// Parsed CLI configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent CLI flags are naturally boolean"
)]
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Config {
    pub(crate) repo: String,
    /// Hostname for the flake attribute, or `None` when it could not be
    /// detected and no `--hostname` was given.
    pub(crate) hostname: Option<String>,
    pub(crate) keep: u8,
    pub(crate) update: bool,
    pub(crate) diff: bool,
    /// Assume "yes" for all confirmation prompts.
    pub(crate) yes: bool,
    /// Print commands instead of executing them.
    pub(crate) dry_run: bool,
    /// `--max-jobs` cap for the rebuild.
    pub(crate) jobs: Option<u16>,
    /// `--cores` cap for the rebuild.
    pub(crate) cores: Option<u16>,
    /// Explicit step selection (`--only`), or `None` for the default pipeline.
    pub(crate) only: Option<Vec<Step>>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ repo: {}, hostname: {}, keep: {}, update: {}, diff: {} }}",
            self.repo,
            self.hostname.as_deref().unwrap_or("<not detected>"),
            self.keep,
            self.update,
            self.diff
        )
    }
}

impl Config {
    /// Check invariants before running the workflow.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.repo.is_empty() {
            return Err(Error::InvalidConfig("repo path cannot be empty"));
        }
        if matches!(self.hostname.as_deref(), Some("")) {
            return Err(Error::InvalidConfig("hostname cannot be empty"));
        }
        if self.keep == 0 {
            return Err(Error::InvalidConfig("generations to keep must be > 0"));
        }
        if self.jobs == Some(0) {
            return Err(Error::InvalidConfig("jobs must be > 0"));
        }
        if self.cores == Some(0) {
            return Err(Error::InvalidConfig("cores must be > 0"));
        }
        Ok(())
    }

    /// Whether `step` belongs to this run.
    ///
    /// `--only` wins when given; otherwise pull/stage/rebuild/cleanup always run
    /// and the flake update / closure diff follow their own flags.
    pub(crate) fn runs(&self, step: Step) -> bool {
        match &self.only {
            Some(steps) => steps.contains(&step),
            None => match step {
                Step::Flake => self.update,
                Step::Diff => self.diff,
                Step::Pull | Step::Stage | Step::Rebuild | Step::Cleanup => true,
            },
        }
    }

    /// Parallelism caps for the rebuild step.
    pub(crate) fn build_options(&self) -> BuildOptions {
        BuildOptions {
            jobs: self.jobs,
            cores: self.cores,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            repo: String::from("repo"),
            hostname: Some(String::from("host")),
            keep: 5,
            update: true,
            diff: false,
            yes: false,
            dry_run: false,
            jobs: None,
            cores: None,
            only: None,
        }
    }

    #[test]
    fn config_display_contains_repo() {
        assert!(sample().to_string().contains("repo"));
    }

    #[test]
    fn config_display_full_format() {
        assert_eq!(
            sample().to_string(),
            "Config { repo: repo, hostname: host, keep: 5, update: true, diff: false }"
        );
    }

    #[test]
    fn validate_accepts_valid_config() {
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_repo() {
        let config = Config {
            repo: String::new(),
            ..sample()
        };
        assert_eq!(
            config.validate().expect_err("empty repo").to_string(),
            "repo path cannot be empty"
        );
    }

    #[test]
    fn validate_rejects_zero_keep() {
        let config = Config {
            keep: 0,
            ..sample()
        };
        assert_eq!(
            config.validate().expect_err("zero keep").to_string(),
            "generations to keep must be > 0"
        );
    }

    #[test]
    fn validate_rejects_empty_hostname() {
        let config = Config {
            hostname: Some(String::new()),
            ..sample()
        };
        assert_eq!(
            config.validate().expect_err("empty hostname").to_string(),
            "hostname cannot be empty"
        );
    }

    #[test]
    fn validate_accepts_undetected_hostname() {
        assert!(
            Config {
                hostname: None,
                ..sample()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn config_display_marks_an_undetected_hostname() {
        let config = Config {
            hostname: None,
            ..sample()
        };
        assert!(config.to_string().contains("<not detected>"));
    }

    #[test]
    fn validate_rejects_zero_jobs() {
        let config = Config {
            jobs: Some(0),
            ..sample()
        };
        assert_eq!(
            config.validate().expect_err("zero jobs").to_string(),
            "jobs must be > 0"
        );
    }

    #[test]
    fn validate_rejects_zero_cores() {
        let config = Config {
            cores: Some(0),
            ..sample()
        };
        assert_eq!(
            config.validate().expect_err("zero cores").to_string(),
            "cores must be > 0"
        );
    }

    #[test]
    fn runs_defaults_to_the_full_pipeline() {
        let config = sample();
        assert!(config.runs(Step::Pull));
        assert!(config.runs(Step::Stage));
        assert!(config.runs(Step::Rebuild));
        assert!(config.runs(Step::Cleanup));
        assert!(config.runs(Step::Flake));
        assert!(!config.runs(Step::Diff));
    }

    #[test]
    fn runs_prefers_an_explicit_selection() {
        let config = Config {
            only: Some(vec![Step::Diff]),
            update: true,
            diff: false,
            ..sample()
        };
        assert!(config.runs(Step::Diff));
        assert!(!config.runs(Step::Pull));
        assert!(!config.runs(Step::Flake));
    }

    #[test]
    fn step_names_round_trip() {
        for step in Step::ALL {
            assert_eq!(Step::from_name(step.name()), Some(step));
        }
        assert!(Step::from_name("bogus").is_none());
        assert_eq!(Step::from_name("update"), Some(Step::Flake));
        assert_eq!(Step::names(), "pull,flake,stage,rebuild,cleanup,diff");
    }

    #[test]
    fn build_options_mirror_jobs_and_cores() {
        let config = Config {
            jobs: Some(2),
            cores: Some(4),
            ..sample()
        };
        assert_eq!(
            config.build_options(),
            BuildOptions {
                jobs: Some(2),
                cores: Some(4)
            }
        );
    }
}
