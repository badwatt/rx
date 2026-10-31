use std::io::{self, BufRead, Write};

use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::shared::ansi::{self, Palette};
use crate::system_update::ports::user_io::UserIo;

/// Production [`UserIo`]: prompts and status on stdout.
///
/// Diagnostics do not flow through this port; top-level errors are reported by
/// `main` on stderr, so a failing step never prints twice.
pub(crate) struct TerminalIo {
    out: Palette,
}

impl TerminalIo {
    pub(crate) fn new(out: Palette) -> Self {
        Self { out }
    }
}

impl UserIo for TerminalIo {
    fn title(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        print_title(&mut out, text, self.out)
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
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "" => default,
        _ => false,
    }
}

/// Print all `config` fields with colored labels, each terminated by a newline.
pub(crate) fn print_config(
    writer: &mut dyn Write,
    config: &Config,
    palette: Palette,
) -> Result<(), Error> {
    print_config_line(writer, "repo", &config.repo, palette)?;
    let hostname = config.hostname.as_deref().unwrap_or("<not detected>");
    print_config_line(writer, "hostname", hostname, palette)?;
    print_config_line(writer, "keep", &config.keep.to_string(), palette)?;
    print_config_line(writer, "update", &config.update().to_string(), palette)?;
    print_config_line(writer, "diff", &config.diff().to_string(), palette)?;
    if let Some(jobs) = config.jobs {
        print_config_line(writer, "jobs", &jobs.to_string(), palette)?;
    }
    if let Some(cores) = config.cores {
        print_config_line(writer, "cores", &cores.to_string(), palette)?;
    }
    // Shown only when set, so the summary never lies about the pipeline.
    if let Some(steps) = config.only() {
        let names: Vec<&str> = steps.iter().map(|step| step.name()).collect();
        print_config_line(writer, "only", &names.join(","), palette)?;
    }
    // Shown only when set, so a dry run is never mistaken for a real one.
    if config.dry_run {
        print_config_line(writer, "dry-run", &config.dry_run.to_string(), palette)?;
    }
    Ok(())
}

fn print_config_line(
    writer: &mut dyn Write,
    label: &str,
    value: &str,
    palette: Palette,
) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!(
            "{}◉ {}{}{} = {}{}{}\n",
            palette.cyan, palette.red, label, palette.reset, palette.cyan, value, palette.reset
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
