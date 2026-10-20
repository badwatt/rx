use super::*;

fn args(from: &[&str]) -> Vec<String> {
    from.iter().copied().map(str::to_string).collect()
}

// ------------------------------------------------------------------
// early-return paths
// ------------------------------------------------------------------

#[test]
fn parse_no_args_yields_defaults() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx"]), &mut buf),
        Intent::Run(default_config())
    );
}

#[test]
fn parse_help_flag_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-h"]), &mut buf), Intent::Help);
}

#[test]
fn parse_help_word_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "help"]), &mut buf), Intent::Help);
}

#[test]
fn parse_version_flag_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-v"]), &mut buf), Intent::Version);
}

#[test]
fn parse_version_word_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "version"]), &mut buf), Intent::Version);
}

#[test]
fn parse_unknown_argument_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "unknown"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("Unknown argument"));
}

// ------------------------------------------------------------------
// value flags
// ------------------------------------------------------------------

#[test]
fn parse_r_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-r"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_n_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-n"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_k_non_numeric_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-k", "abc"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
}

#[test]
fn parse_k_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-k", "0"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("at least 1"));
}

#[test]
fn parse_r_with_value_sets_repo() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/path/to/repo");
    assert_eq!(
        parse(&args(&["rx", "-r", "/path/to/repo"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_n_with_value_sets_hostname() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.hostname = String::from("myhost");
    assert_eq!(
        parse(&args(&["rx", "-n", "myhost"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_k_with_value_sets_keep() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.keep = 5;
    assert_eq!(
        parse(&args(&["rx", "-k", "5"]), &mut buf),
        Intent::Run(expected)
    );
}

// ------------------------------------------------------------------
// boolean flags
// ------------------------------------------------------------------

#[test]
fn parse_d_flag_sets_diff() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.diff = true;
    assert_eq!(parse(&args(&["rx", "-d"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_u_flag_sets_update() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    assert_eq!(parse(&args(&["rx", "-u"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_y_flag_sets_yes() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    assert_eq!(parse(&args(&["rx", "-y"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_d_and_u_together() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    expected.diff = true;
    assert_eq!(
        parse(&args(&["rx", "-d", "-u"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_combined_flags_and_values() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/my/repo");
    expected.hostname = String::from("myhost");
    expected.keep = 3;
    expected.update = true;
    expected.diff = true;
    assert_eq!(
        parse(
            &args(&[
                "rx", "-d", "-u", "-r", "/my/repo", "-n", "myhost", "-k", "3",
            ]),
            &mut buf,
        ),
        Intent::Run(expected)
    );
}

#[test]
fn parse_k_then_h_early_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-k", "5", "-h"]), &mut buf),
        Intent::Help
    );
}

#[test]
fn parse_unknown_flag_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-x"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
}

#[test]
fn parse_short_value_flag_must_be_last_in_cluster() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-rk", "/r"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("must be last"));
}

#[test]
fn parse_short_boolean_then_value_in_cluster_ok() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    expected.repo = String::from("/r");
    assert_eq!(
        parse(&args(&["rx", "-yr", "/r"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_short_value_rejects_flag_like_value() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-r", "-d"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_long_value_rejects_flag_like_value() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--repo", "--diff"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_extra_non_flag_arg_ignored() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.diff = true;
    assert_eq!(
        parse(&args(&["rx", "-d", "extra"]), &mut buf),
        Intent::Run(expected)
    );
}

// ------------------------------------------------------------------
// long flags
// ------------------------------------------------------------------

#[test]
fn parse_help_long_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--help"]), &mut buf), Intent::Help);
}

#[test]
fn parse_version_long_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--version"]), &mut buf),
        Intent::Version
    );
}

#[test]
fn parse_update_long_sets_update() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    assert_eq!(
        parse(&args(&["rx", "--update"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_diff_long_sets_diff() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.diff = true;
    assert_eq!(
        parse(&args(&["rx", "--diff"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_yes_long_sets_yes() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    assert_eq!(
        parse(&args(&["rx", "--yes"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_dry_run_long_sets_dry_run() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.dry_run = true;
    assert_eq!(
        parse(&args(&["rx", "--dry-run"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_repo_long_sets_repo() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/my/repo");
    assert_eq!(
        parse(&args(&["rx", "--repo", "/my/repo"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_hostname_long_sets_hostname() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.hostname = String::from("myhost");
    assert_eq!(
        parse(&args(&["rx", "--hostname", "myhost"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_keep_long_sets_keep() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.keep = 5;
    assert_eq!(
        parse(&args(&["rx", "--keep", "5"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_keep_long_non_numeric_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--keep", "abc"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
}

#[test]
fn parse_keep_long_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--keep", "0"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("at least 1"));
}

#[test]
fn parse_repo_long_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--repo"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_unknown_long_flag_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--unknown"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
}
