//! Supervision of the `rx` CLI.
//!
//! The desktop surface does not re-implement the NixOS workflow: it runs the
//! installed `rx` once per step (`rx --only <step> -y`), which keeps the CLI's
//! fail-fast semantics, its configuration validation and its argv-based
//! commands exactly as shipped.  The GUI only decides *when* a step runs and
//! renders what the step wrote to stdout/stderr.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// One step of the update pipeline, in execution order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    Pull,
    Flake,
    Stage,
    Rebuild,
    Cleanup,
    Diff,
}

impl Step {
    pub(crate) const ALL: [Step; 6] = [
        Step::Pull,
        Step::Flake,
        Step::Stage,
        Step::Rebuild,
        Step::Cleanup,
        Step::Diff,
    ];

    /// `--only` name, shared with the CLI.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Step::Pull => "pull",
            Step::Flake => "flake",
            Step::Stage => "stage",
            Step::Rebuild => "rebuild",
            Step::Cleanup => "cleanup",
            Step::Diff => "diff",
        }
    }

    /// One-line form of a step's command, for the rail.
    pub(crate) fn short_command(self) -> String {
        match self {
            Step::Pull => String::from("git pull --ff-only"),
            Step::Flake => String::from("nix flake update"),
            Step::Stage => String::from("git add ."),
            Step::Rebuild => String::from("nixos-rebuild switch"),
            Step::Cleanup => String::from("prune generations"),
            Step::Diff => String::from("diff closures"),
        }
    }

    /// Headline when this step is the one that stopped the run.
    pub(crate) fn failure_headline(self) -> &'static str {
        match self {
            Step::Pull => "Could not pull",
            Step::Flake => "Flake update failed",
            Step::Stage => "Could not stage",
            Step::Rebuild => "Rebuild failed",
            Step::Cleanup => "Cleanup failed",
            Step::Diff => "Diff failed",
        }
    }

    /// Human label shown in the rail.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Step::Pull => "Pull dotfiles",
            Step::Flake => "Update flake inputs",
            Step::Stage => "Stage changes",
            Step::Rebuild => "Rebuild system",
            Step::Cleanup => "Clean up generations",
            Step::Diff => "Closure diff",
        }
    }

    /// What the step actually does, for the confirmation sheet.
    pub(crate) fn detail(self) -> &'static str {
        match self {
            Step::Pull => "git pull --ff-only, without hanging on credentials",
            Step::Flake => "nix flake update, locking the newest inputs",
            Step::Stage => "git add . so the rebuild sees the working tree",
            Step::Rebuild => "nixos-rebuild switch, replacing the running system",
            Step::Cleanup => "delete old generations, then nix store gc",
            Step::Diff => "nix store diff-closures, old generation against new",
        }
    }
}

/// Everything the CLI needs to run one step.
#[derive(Clone, Debug)]
pub(crate) struct RunConfig {
    pub(crate) repo: String,
    pub(crate) hostname: String,
    pub(crate) keep: u8,
    pub(crate) jobs: Option<u16>,
    pub(crate) cores: Option<u16>,
    pub(crate) update: bool,
    pub(crate) diff: bool,
    pub(crate) dry_run: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            repo: if home.is_empty() {
                String::from("~/.dotfiles")
            } else {
                format!("{home}/.dotfiles")
            },
            hostname: detect_hostname().unwrap_or_default(),
            keep: 10,
            jobs: None,
            cores: None,
            update: false,
            diff: false,
            dry_run: false,
        }
    }
}

impl RunConfig {
    /// The steps that will run, in order.  Flake and diff are opt-in.
    pub(crate) fn plan(&self) -> Vec<Step> {
        Step::ALL
            .into_iter()
            .filter(|step| match step {
                Step::Flake => self.update,
                Step::Diff => self.diff,
                _ => true,
            })
            .collect()
    }

    /// The argv handed to `rx` for one step.
    pub(crate) fn rx_args(&self, step: Step) -> Vec<String> {
        let mut args = vec![
            String::from("--only"),
            String::from(step.name()),
            String::from("--yes"),
            String::from("--no-color"),
            String::from("--repo"),
            self.repo.clone(),
            String::from("--hostname"),
            self.hostname.clone(),
            String::from("--keep"),
            self.keep.to_string(),
        ];
        if let Some(jobs) = self.jobs {
            args.push(String::from("--jobs"));
            args.push(jobs.to_string());
        }
        if let Some(cores) = self.cores {
            args.push(String::from("--cores"));
            args.push(cores.to_string());
        }
        if self.dry_run {
            args.push(String::from("--dry-run"));
        }
        args
    }

    /// The underlying command of a step, for the confirmation sheet.
    pub(crate) fn command(&self, step: Step) -> String {
        match step {
            Step::Pull => String::from("git pull --ff-only"),
            Step::Flake => String::from("nix flake update"),
            Step::Stage => String::from("git add ."),
            Step::Rebuild => format!("nixos-rebuild switch --flake .#{}", self.hostname),
            Step::Cleanup => format!(
                "nix-env --delete-generations +{} && nix store gc",
                self.keep
            ),
            Step::Diff => String::from("nix store diff-closures"),
        }
    }

    /// Does any planned step need root?
    pub(crate) fn needs_root(&self) -> bool {
        self.plan().contains(&Step::Rebuild)
    }
}

/// Messages from the supervisor thread to the UI.
pub(crate) enum Msg {
    Started(Step),
    Line(LineKind, String),
    Finished(Step, Outcome, Duration),
    Done(Outcome, Duration),
}

/// Result of one step, named instead of a bare `bool`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Outcome {
    Ok,
    Failed,
}

impl Outcome {
    pub(crate) fn is_ok(self) -> bool {
        self == Self::Ok
    }
}

/// How a log line reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LineKind {
    Plain,
    Section,
    Good,
    Warn,
    Bad,
}

/// Locate the `rx` binary: first on `PATH`, then next to this executable.
pub(crate) fn find_rx() -> Option<PathBuf> {
    let exe = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("rx"))
            .find(|candidate| candidate.is_file())
    });
    exe.or_else(|| {
        let sibling = std::env::current_exe().ok()?.with_file_name("rx");
        sibling.is_file().then_some(sibling)
    })
}

/// Best-effort hostname, mirroring the CLI default.
pub(crate) fn detect_hostname() -> Option<String> {
    let output = Command::new("hostname").output().ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!name.is_empty()).then_some(name)
}

/// Is a sudo timestamp already valid?  Never prompts.
pub(crate) fn root_is_ready() -> bool {
    Command::new("sudo")
        .args(["-n", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Validate the sudo timestamp with `sudo -S -p '' -v`.
///
/// The password travels on stdin only: never in argv, never in the
/// environment, both of which are world-readable through `/proc`.
pub(crate) fn authenticate(password: &str) -> bool {
    let spawned = Command::new("sudo")
        .args(["-S", "-p", "", "-v"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = spawned else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes());
        let _ = stdin.write_all(b"\n");
    }
    child.wait().is_ok_and(|status| status.success())
}

/// Effective uid, read from `/proc/self/status` (no libc dependency).
fn current_uid() -> Option<u32> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|uid| uid.parse().ok())
}

/// Directory that holds the askpass secret: the user's runtime dir, or the
/// temp dir when there is none.
fn secret_dir(uid: u32) -> PathBuf {
    let runtime = PathBuf::from(format!("/run/user/{uid}"));
    if runtime.is_dir() {
        runtime
    } else {
        std::env::temp_dir()
    }
}

/// Where the askpass helper looks for the secret.
///
/// Derived from the uid on both sides: sudo resets the environment before
/// executing a command, so an env var passed to `rx` cannot be trusted.
pub(crate) fn secret_path() -> Option<PathBuf> {
    let uid = current_uid()?;
    Some(secret_dir(uid).join(format!("rxui-sudo-{uid}")))
}

/// Stash the password for `sudo`'s askpass helper at `dir`.
///
/// Created at `0600` from the first byte; the caller removes it when the run
/// ends.  `dir` is a parameter so tests never touch the live stash.
pub(crate) fn stash_secret_in(dir: &Path, uid: u32, password: &str) -> Option<PathBuf> {
    let path = dir.join(format!("rxui-sudo-{uid}"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).ok()?;
    file.write_all(password.as_bytes()).ok()?;
    file.flush().ok()?;
    Some(path)
}

/// Stash the password where `rxui --askpass` will look for it.
pub(crate) fn stash_secret(password: &str) -> Option<PathBuf> {
    let uid = current_uid()?;
    stash_secret_in(&secret_dir(uid), uid, password)
}

/// Best-effort wipe of a secret from its allocation before dropping it.
pub(crate) fn wipe(secret: &mut String) {
    let len = secret.len();
    if len > 0 {
        secret.replace_range(.., &"x".repeat(len));
    }
    secret.clear();
}

/// Keep the sudo timestamp alive while a long rebuild runs.
///
/// `sudo -n -v` refreshes it without a password; without this a rebuild that
/// outlives `timestamp_timeout` makes the nested `sudo nixos-rebuild` prompt
/// again — with no terminal to answer it.
struct KeepAlive {
    stop: Arc<AtomicBool>,
}

impl KeepAlive {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                let _ = Command::new("sudo")
                    .args(["-n", "-v"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                for _ in 0..600 {
                    if flag.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Self { stop }
    }
}

impl Drop for KeepAlive {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Run every planned step in order, reporting to `tx`.
pub(crate) fn run_all(cfg: RunConfig, binary: PathBuf, askpass: Option<PathBuf>, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let total = Instant::now();
        let keepalive = askpass.as_ref().map(|_| KeepAlive::start());
        let mut all_ok = true;
        for step in cfg.plan() {
            let _ = tx.send(Msg::Started(step));
            let started = Instant::now();
            let outcome = run_step(&cfg, step, &binary, askpass.as_deref(), &tx);
            let _ = tx.send(Msg::Finished(step, outcome, started.elapsed()));
            if !outcome.is_ok() {
                all_ok = false;
                break;
            }
        }
        drop(keepalive);
        if let Some(path) = &askpass {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(
                path.with_file_name(format!("rxui-askpass-{}", std::process::id())),
            );
        }
        let _ = tx.send(Msg::Done(outcome_of(all_ok), total.elapsed()));
    });
}

/// Run a single `rx --only <step>` and stream its output.
fn run_step(
    cfg: &RunConfig,
    step: Step,
    binary: &PathBuf,
    askpass: Option<&std::path::Path>,
    tx: &Sender<Msg>,
) -> Outcome {
    let mut command = Command::new(binary);
    command
        .args(cfg.rx_args(step))
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Without a terminal, sudo runs this helper instead of hanging.
    if askpass.is_some()
        && let Ok(program) = std::env::current_exe()
    {
        command.env("SUDO_ASKPASS", program);
    }
    let spawned = command.spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            let _ = tx.send(Msg::Line(
                LineKind::Bad,
                format!("cannot start {}: {error}", binary.display()),
            ));
            return Outcome::Failed;
        }
    };

    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(spawn_reader(Stream::Out(stdout), tx.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(spawn_reader(Stream::Err(stderr), tx.clone()));
    }

    let status = child.wait();
    for reader in readers {
        let _ = reader.join();
    }
    if status.is_ok_and(|exit| exit.success()) {
        Outcome::Ok
    } else {
        Outcome::Failed
    }
}

/// Bridge from the boolean accumulator to the enum.
fn outcome_of(ok: bool) -> Outcome {
    if ok { Outcome::Ok } else { Outcome::Failed }
}

enum Stream {
    Out(ChildStdout),
    Err(ChildStderr),
}

fn spawn_reader(stream: Stream, tx: Sender<Msg>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || match stream {
        Stream::Out(reader) => pump(reader, &tx),
        Stream::Err(reader) => pump(reader, &tx),
    })
}

fn pump(reader: impl Read, tx: &Sender<Msg>) {
    for line in BufReader::new(reader).lines() {
        let Ok(raw) = line else { break };
        let Some((kind, text)) = clean(&raw) else {
            continue;
        };
        if tx.send(Msg::Line(kind, text)).is_err() {
            break;
        }
    }
}

/// Strip rx's own chrome and terminal control codes, then classify.
fn clean(raw: &str) -> Option<(LineKind, String)> {
    // Progress bars redraw with `\r`; keep only the final state of the line.
    let line = raw.rsplit('\r').next().unwrap_or(raw);
    let text = strip_ansi(line);
    let text = text.trim_end().to_owned();
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.chars().all(|c| c == '*')
        || (trimmed.starts_with('*') && trimmed.ends_with('*'))
        || trimmed.starts_with('◉')
        || trimmed.starts_with("Proceed?")
        || trimmed.ends_with("(Y/n):")
        || trimmed.ends_with("(y/N):")
    {
        return None;
    }
    Some((classify(trimmed), text))
}

fn classify(text: &str) -> LineKind {
    let lower = text.to_lowercase();
    if lower.contains("error")
        || lower.contains("failed")
        || lower.contains("aborting")
        || lower.starts_with("error:")
    {
        LineKind::Bad
    } else if lower.contains("warning") {
        LineKind::Warn
    } else if lower.contains("successfully") || lower.contains("added to git stage") {
        LineKind::Good
    } else {
        LineKind::Plain
    }
}

/// Drop CSI/OSC escape sequences from a line.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            // CSI: ESC [ parameters final-byte.
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            // OSC: ESC ] ... BEL or ST.
            Some(']') => {
                chars.next();
                for c in chars.by_ref() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' {
                        let _ = chars.next();
                        break;
                    }
                }
            }
            _ => {
                let _ = chars.next();
            }
        }
    }
    out
}

/// Format a duration as a human-friendly token: `0.4s`, `12.8s`, `1m 04s`.
pub(crate) fn clock(duration: Duration) -> String {
    let secs = duration.as_secs_f32();
    if secs < 60. {
        format!("{secs:.1}s")
    } else {
        let minutes = duration.as_secs() / 60;
        let rest = duration.as_secs() % 60;
        format!("{minutes}m {rest:02}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_sequences() {
        assert_eq!(strip_ansi("\u{1b}[1;32mok\u{1b}[0m"), "ok");
    }

    #[test]
    fn strip_ansi_removes_osc_titles() {
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}body"), "body");
    }

    #[test]
    fn strip_ansi_keeps_plain_text() {
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn clean_drops_rx_banners() {
        assert!(clean("**********").is_none());
        assert!(clean("* Git Pull *").is_none());
    }

    #[test]
    fn clean_drops_config_lines() {
        assert!(clean("◉ repo = ~/.dotfiles").is_none());
    }

    #[test]
    fn clean_keeps_child_output() {
        let (kind, text) = clean("building '/nix/store/x.drv'...").expect("line");
        assert_eq!(kind, LineKind::Plain);
        assert_eq!(text, "building '/nix/store/x.drv'...");
    }

    #[test]
    fn clean_keeps_last_carriage_return_frame() {
        let (_, text) = clean("10%\r50%\r100%").expect("line");
        assert_eq!(text, "100%");
    }

    #[test]
    fn classify_marks_failures() {
        assert_eq!(classify("error: boom"), LineKind::Bad);
        assert_eq!(classify("rebuild failed"), LineKind::Bad);
    }

    #[test]
    fn classify_marks_warnings_and_success() {
        assert_eq!(classify("warning: unused"), LineKind::Warn);
        assert_eq!(
            classify("Changes added to git stage successfully"),
            LineKind::Good
        );
    }

    #[test]
    fn clock_formats_seconds_and_minutes() {
        assert_eq!(clock(Duration::from_millis(400)), "0.4s");
        assert_eq!(clock(Duration::from_secs(64)), "1m 04s");
    }

    #[test]
    fn plan_skips_optional_steps_by_default() {
        let cfg = RunConfig::default();
        assert_eq!(
            cfg.plan(),
            vec![Step::Pull, Step::Stage, Step::Rebuild, Step::Cleanup]
        );
    }

    #[test]
    fn plan_includes_optional_steps_when_enabled() {
        let cfg = RunConfig {
            update: true,
            diff: true,
            ..RunConfig::default()
        };
        assert_eq!(cfg.plan(), Step::ALL.to_vec());
    }

    #[test]
    fn rx_args_select_one_step_with_yes() {
        let cfg = RunConfig {
            repo: String::from("/tmp/dotfiles"),
            hostname: String::from("host"),
            ..RunConfig::default()
        };
        let args = cfg.rx_args(Step::Pull);
        assert!(args.contains(&String::from("--only")));
        assert!(args.contains(&String::from("pull")));
        assert!(args.contains(&String::from("--yes")));
        assert!(args.contains(&String::from("/tmp/dotfiles")));
    }

    #[test]
    fn stash_secret_keeps_the_secret_owner_readable_only() {
        let dir = std::env::temp_dir().join(format!("rxui-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = stash_secret_in(&dir, 4242, "hunter2").expect("stash");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "hunter2");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn wipe_overwrites_the_secret_before_clearing() {
        let mut secret = String::from("hunter2");
        wipe(&mut secret);
        assert!(secret.is_empty());
        assert_ne!(secret.capacity(), 0);
    }

    #[test]
    fn command_is_the_underlying_argv() {
        let cfg = RunConfig {
            hostname: String::from("host"),
            keep: 7,
            ..RunConfig::default()
        };
        assert_eq!(cfg.command(Step::Pull), "git pull --ff-only");
        assert_eq!(
            cfg.command(Step::Rebuild),
            "nixos-rebuild switch --flake .#host"
        );
        assert!(cfg.command(Step::Cleanup).contains("+7"));
    }

    #[test]
    fn needs_root_tracks_the_rebuild_step() {
        let cfg = RunConfig::default();
        assert!(cfg.needs_root());
    }

    #[test]
    fn rx_args_pass_build_caps_when_set() {
        let cfg = RunConfig {
            jobs: Some(4),
            cores: Some(8),
            dry_run: true,
            ..RunConfig::default()
        };
        let args = cfg.rx_args(Step::Rebuild);
        assert!(args.contains(&String::from("--jobs")));
        assert!(args.contains(&String::from("--cores")));
        assert!(args.contains(&String::from("--dry-run")));
    }
}
