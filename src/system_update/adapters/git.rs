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
        let (status, prefix) = self.runner.capture(&work_tree_prefix(repo))?;
        // An empty prefix means `repo` *is* the work-tree root. A subdirectory
        // would pass a plain "is inside a work tree" check while `pull`/`add`
        // then act on the enclosing repository.
        Ok(status == 0 && prefix.trim().is_empty())
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

/// `git -C <repo> rev-parse --show-prefix` (empty at the work-tree root)
fn work_tree_prefix(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "rev-parse".into(),
        "--show-prefix".into(),
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
    fn work_tree_prefix_argv() {
        assert_eq!(
            work_tree_prefix("/repo"),
            vec!["git", "-C", "/repo", "rev-parse", "--show-prefix"]
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
    fn is_repo_false_for_a_subdirectory_of_a_work_tree() {
        let runner = StubRunner::new().with_capture(|_| Ok((0, String::from("nixos/\n"))));
        assert!(
            !GitRepo::new(&runner)
                .is_repo("/repo/nixos")
                .expect("is_repo")
        );
    }

    #[test]
    fn is_repo_false_when_status_nonzero() {
        let runner = StubRunner::new().with_capture(|_| Ok((1, String::new())));
        assert!(!GitRepo::new(&runner).is_repo("/repo").expect("is_repo"));
    }

    #[test]
    fn is_repo_propagates_capture_error() {
        let runner =
            StubRunner::new().with_capture(|_| Err(Error::Io(std::io::Error::other("boom"))));
        let err = GitRepo::new(&runner)
            .is_repo("/repo")
            .expect_err("capture error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn pull_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(GitRepo::new(&runner).pull("/repo").is_ok());
    }

    #[test]
    fn pull_errs_on_nonzero() {
        let runner = StubRunner::new().with_run_env(|_, _, _| Ok(1));
        let err = GitRepo::new(&runner)
            .pull("/repo")
            .expect_err("pull failure");
        assert_eq!(err.to_string(), "Failed to pull changes");
    }

    #[test]
    fn pull_propagates_run_error() {
        let runner =
            StubRunner::new().with_run_env(|_, _, _| Err(Error::Io(std::io::Error::other("boom"))));
        let err = GitRepo::new(&runner).pull("/repo").expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn changes_returns_output_on_zero() {
        let runner = StubRunner::new();
        assert_eq!(GitRepo::new(&runner).changes("/repo").expect("changes"), "");
    }

    #[test]
    fn changes_errs_on_nonzero() {
        let runner = StubRunner::new().with_capture(|_| Ok((1, String::new())));
        let err = GitRepo::new(&runner)
            .changes("/repo")
            .expect_err("status failure");
        assert_eq!(err.to_string(), "Failed to read repository status");
    }

    #[test]
    fn changes_propagates_capture_error() {
        let runner =
            StubRunner::new().with_capture(|_| Err(Error::Io(std::io::Error::other("boom"))));
        let err = GitRepo::new(&runner)
            .changes("/repo")
            .expect_err("capture error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn add_all_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(GitRepo::new(&runner).add_all("/repo").is_ok());
    }

    #[test]
    fn add_all_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = GitRepo::new(&runner)
            .add_all("/repo")
            .expect_err("add failure");
        assert_eq!(err.to_string(), "Failed to add changes to the stage");
    }

    #[test]
    fn add_all_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("boom"))));
        let err = GitRepo::new(&runner)
            .add_all("/repo")
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }
}
