use crate::domain::error::Error;
use crate::infra::command::CommandRunner;
use crate::system_update::ports::repo::Repo;

/// [`Repo`] adapter backed by the `git` CLI.
pub(crate) struct GitRepo<'a> {
    runner: &'a dyn CommandRunner,
}

impl<'a> GitRepo<'a> {
    pub(crate) fn new(runner: &'a dyn CommandRunner) -> Self {
        Self { runner }
    }
}

impl Repo for GitRepo<'_> {
    fn is_repo(&self, repo: &str) -> Result<bool, Error> {
        let (status, _) = self.runner.capture(&rev_parse(repo))?;
        Ok(status == 0)
    }

    fn pull(&self, repo: &str) -> Result<(), Error> {
        // `--ff-only` keeps a diverged branch from opening a merge editor and
        // `GIT_TERMINAL_PROMPT=0` keeps missing credentials from prompting.
        let status = self
            .runner
            .run_with_env(&pull(repo), "GIT_TERMINAL_PROMPT", "0")?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::GitPullFailed)
        }
    }

    fn changes(&self, repo: &str) -> Result<String, Error> {
        let (status, output) = self.runner.capture(&status(repo))?;
        if status == 0 {
            Ok(output)
        } else {
            Err(Error::GitStatusFailed)
        }
    }

    fn add_all(&self, repo: &str) -> Result<(), Error> {
        let status = self.runner.run(&add(repo))?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::GitAddFailed)
        }
    }
}

/// `git -C <repo> pull --ff-only`
fn pull(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "pull".into(),
        "--ff-only".into(),
    ]
}

/// `git -C <repo> status --porcelain`
fn status(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "status".into(),
        "--porcelain".into(),
    ]
}

/// `git -C <repo> rev-parse --is-inside-work-tree`
fn rev_parse(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "rev-parse".into(),
        "--is-inside-work-tree".into(),
    ]
}

/// `git -C <repo> add .`
fn add(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "add".into(),
        ".".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers::StubRunner;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn pull_argv() {
        assert_eq!(
            pull("/repo"),
            vec!["git", "-C", "/repo", "pull", "--ff-only"]
        );
    }

    #[test]
    fn pull_disables_terminal_prompts() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new().with_run_env(move |_, key, value| {
            recorder.borrow_mut().push(format!("{key}={value}"));
            Ok(0)
        });
        assert!(GitRepo::new(&runner).pull("/repo").is_ok());
        assert_eq!(seen.borrow().join(" "), "GIT_TERMINAL_PROMPT=0");
    }

    #[test]
    fn status_argv() {
        assert_eq!(
            status("/repo"),
            vec!["git", "-C", "/repo", "status", "--porcelain"]
        );
    }

    #[test]
    fn rev_parse_argv() {
        assert_eq!(
            rev_parse("/repo"),
            vec!["git", "-C", "/repo", "rev-parse", "--is-inside-work-tree"]
        );
    }

    #[test]
    fn add_argv() {
        assert_eq!(add("/repo"), vec!["git", "-C", "/repo", "add", "."]);
    }

    #[test]
    fn is_repo_true_when_status_zero() {
        let runner = StubRunner::new();
        assert!(GitRepo::new(&runner).is_repo("/repo").expect("is_repo"));
    }

    #[test]
    fn is_repo_false_when_status_nonzero() {
        let runner = StubRunner::new().with_capture(|_| Ok((1, String::new())));
        assert!(!GitRepo::new(&runner).is_repo("/repo").expect("is_repo"));
    }

    #[test]
    fn is_repo_propagates_capture_error() {
        let runner = StubRunner::new().with_capture(|_| Err(Error::GitStatusFailed));
        assert!(GitRepo::new(&runner).is_repo("/repo").is_err());
    }

    #[test]
    fn pull_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(GitRepo::new(&runner).pull("/repo").is_ok());
    }

    #[test]
    fn pull_errs_on_nonzero() {
        let runner = StubRunner::new().with_run_env(|_, _, _| Ok(1));
        assert!(GitRepo::new(&runner).pull("/repo").is_err());
    }

    #[test]
    fn pull_propagates_run_error() {
        let runner = StubRunner::new().with_run_env(|_, _, _| Err(Error::GitPullFailed));
        assert!(GitRepo::new(&runner).pull("/repo").is_err());
    }

    #[test]
    fn changes_returns_output_on_zero() {
        let runner = StubRunner::new();
        assert_eq!(GitRepo::new(&runner).changes("/repo").expect("changes"), "");
    }

    #[test]
    fn changes_errs_on_nonzero() {
        let runner = StubRunner::new().with_capture(|_| Ok((1, String::new())));
        assert!(GitRepo::new(&runner).changes("/repo").is_err());
    }

    #[test]
    fn changes_propagates_capture_error() {
        let runner = StubRunner::new().with_capture(|_| Err(Error::GitStatusFailed));
        assert!(GitRepo::new(&runner).changes("/repo").is_err());
    }

    #[test]
    fn add_all_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(GitRepo::new(&runner).add_all("/repo").is_ok());
    }

    #[test]
    fn add_all_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        assert!(GitRepo::new(&runner).add_all("/repo").is_err());
    }

    #[test]
    fn add_all_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::GitAddFailed));
        assert!(GitRepo::new(&runner).add_all("/repo").is_err());
    }
}
