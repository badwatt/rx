use std::io::{self, BufRead, IsTerminal, Write};

use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::shared::ansi::{self, Palette};
use crate::shared::osc::{self, State};
use crate::shared::progress::Progress;
use crate::system_update::ports::user_io::UserIo;

/// Production [`UserIo`]: prompts and status on stdout.
///
/// Diagnostics do not flow through this port; top-level errors are reported by
/// `main` on stderr, so a failing step never prints twice.
pub(crate) struct TerminalIo {
    out: Palette,
    /// Whether OSC 7501 reports and the OSC 9;4 bar may be written: an
    /// interactive terminal that is not a dry run.
    osc: bool,
    /// The tab progress bar, driven by the same state reports.
    bar: Progress,
}

impl TerminalIo {
    pub(crate) fn new(out: Palette, dry_run: bool) -> Self {
        Self::detect(out, io::stdout().is_terminal(), dry_run)
    }

    /// Constructor with the TTY gate pinned, so tests cover both sides.
    fn detect(out: Palette, is_tty: bool, dry_run: bool) -> Self {
        let term = std::env::var("TERM").ok();
        let osc = osc_enabled(is_tty, dry_run, term.as_deref());
        Self {
            out,
            osc,
            bar: Progress::new(osc),
        }
    }
}

/// A status report only makes sense for a real terminal that is not `dumb`,
/// and a dry run must not claim the workflow is running.
pub(crate) fn osc_enabled(is_tty: bool, dry_run: bool, term: Option<&str>) -> bool {
    is_tty && !dry_run && term != Some("dumb")
}

impl UserIo for TerminalIo {
    fn title(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        print_title(&mut out, text, self.out)
    }

    fn progress(&self, state: State, msg: &str) -> Result<(), Error> {
        if !self.osc {
            return Ok(());
        }
        // The bar follows the state: working shows it, every moment that needs
        // the user hides it again, and the end clears it.
        match state {
            State::Working => self.bar.show(),
            State::BlockedPermission
            | State::BlockedAuth
            | State::Done
            | State::Failed
            | State::Idle => {
                self.bar.hide();
            }
        }
        let mut out = io::stdout();
        osc::report(&mut out, state, msg)
    }

    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut out = io::stdout();
        confirm(&mut reader, &mut out, default, msg, self.out)
    }

    fn success(&self, msg: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_line(
            &mut out,
            &format!("{}{}{}", self.out.green, msg, self.out.reset),
        )
    }

    fn output(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_raw(&mut out, text)
    }

    fn config(&self, config: &Config) -> Result<(), Error> {
        let mut out = io::stdout();
        print_config(&mut out, config, self.out)
    }
}

fn write_line(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("{text}\n"))?;
    Ok(())
}

fn write_raw(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
    ansi::write_flush(writer, text)?;
    Ok(())
}

/// Print a banner title with colored borders.
///
/// Note: `text.len()` counts bytes, not graphemes. Non-ASCII text
/// will misalign borders. Acceptable for ASCII-only CLI output.
pub(crate) fn print_title(
    writer: &mut dyn Write,
    text: &str,
    palette: Palette,
) -> Result<(), Error> {
    let border: String = "*".repeat(text.len() + 4);
    ansi::write_flush(
        writer,
        &format!(
            "{}\n{}\n* {}{}{} *\n{}\n{}",
            palette.blue, border, palette.red, text, palette.blue, border, palette.reset
        ),
    )?;
    Ok(())
}

/// Prompt user for confirmation on `reader`/`writer`.
///
/// `default` is returned when the user sends an empty line; a closed stdin
/// (EOF) aborts instead, so an empty pipe never confirms.
pub(crate) fn confirm(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    default: bool,
    msg: Option<&str>,
    palette: Palette,
) -> Result<bool, Error> {
    write_confirm_prompt(writer, default, msg, palette)?;
    let mut line = String::new();
    // A closed stdin is not an answer: taking the default here would rebuild
    // the system on an empty pipe (`rx </dev/null`).
    if reader.read_line(&mut line)? == 0 {
        return Ok(false);
    }
    Ok(parse_confirm_response(line.trim(), default))
}

fn write_confirm_prompt(
    writer: &mut dyn Write,
    default: bool,
    msg: Option<&str>,
    palette: Palette,
) -> Result<(), Error> {
    let hint = if default {
        format!("{}(Y/n){}", palette.green, palette.reset)
    } else {
        format!("{}(y/N){}", palette.red, palette.reset)
    };
    let text = match msg {
        Some(value) => format!("\n{}{}{} {}: ", palette.yellow, value, palette.reset, hint),
        None => format!("\n{}Proceed?{} {}: ", palette.yellow, palette.reset, hint),
    };
    ansi::write_flush(writer, &text)?;
    Ok(())
}

/// Parse a confirmation response line.
///
/// Returns `default` for empty input, `true` for "y"/"yes", `false` for
/// "n"/"no" or any other input.
pub(crate) fn parse_confirm_response(line: &str, default: bool) -> bool {
    let answer = line.trim();
    if answer.is_empty() {
        return default;
    }
    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
}

/// Print all `config` fields, each terminated by a newline, in one write.
pub(crate) fn print_config(
    writer: &mut dyn Write,
    config: &Config,
    palette: Palette,
) -> Result<(), Error> {
    let mut text = String::new();
    push_config_line(&mut text, "repo", &config.repo, palette);
    let hostname = config.hostname.as_deref().unwrap_or("<not detected>");
    push_config_line(&mut text, "hostname", hostname, palette);
    push_config_line(&mut text, "keep", &config.keep.to_string(), palette);
    push_config_line(&mut text, "update", &config.update().to_string(), palette);
    push_config_line(&mut text, "diff", &config.diff().to_string(), palette);
    if let Some(jobs) = config.jobs {
        push_config_line(&mut text, "jobs", &jobs.to_string(), palette);
    }
    if let Some(cores) = config.cores {
        push_config_line(&mut text, "cores", &cores.to_string(), palette);
    }
    // Shown only when set, so the summary never lies about the pipeline.
    if let Some(steps) = config.only() {
        let names: Vec<&str> = steps.iter().map(|step| step.name()).collect();
        push_config_line(&mut text, "only", &names.join(","), palette);
    }
    // Shown only when set, so a dry run is never mistaken for a real one.
    if config.dry_run {
        push_config_line(&mut text, "dry-run", &config.dry_run.to_string(), palette);
    }
    write_raw(writer, &text)
}

/// Append one `◉ label = value` line to `out`.
fn push_config_line(out: &mut String, label: &str, value: &str, palette: Palette) {
    use std::fmt::Write as _;
    // Formatting into a `String` cannot fail.
    writeln!(
        out,
        "{}◉ {}{}{} = {}{}{}",
        palette.cyan, palette.red, label, palette.reset, palette.cyan, value, palette.reset
    )
    .expect("writing to a String cannot fail");
}

#[cfg(test)]
mod tests;
