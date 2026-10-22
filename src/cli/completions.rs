//! Shell completions, printed by `rx --completions <shell>`.
//!
//! The scripts are static so the binary stays dependency free: no completion
//! generator crate, no build step, and no runtime parsing beyond the shell name.

use std::fmt;
use std::io::Write;

use crate::domain::error::Error;
use crate::shared::ansi;

/// Shell flavours we can emit completions for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    /// Every supported shell, for help and error messages.
    pub(crate) const ALL: [Shell; 3] = [Shell::Bash, Shell::Zsh, Shell::Fish];

    /// CLI name of the shell.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    }

    /// Look a shell up by its CLI name.
    pub(crate) fn from_name(name: &str) -> Option<Shell> {
        Shell::ALL.into_iter().find(|shell| shell.name() == name)
    }

    /// Comma-separated shell names, for error messages.
    pub(crate) fn names() -> String {
        Shell::ALL
            .iter()
            .map(|shell| shell.name())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The completion script for this shell.
    pub(crate) fn script(self) -> &'static str {
        match self {
            Shell::Bash => BASH,
            Shell::Zsh => ZSH,
            Shell::Fish => FISH,
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Write the completion script for `shell`.
pub(crate) fn print(writer: &mut dyn Write, shell: Shell) -> Result<(), Error> {
    ansi::write_flush(writer, shell.script())?;
    Ok(())
}

const BASH: &str = r#"# bash completion for rx
# Install: rx --completions bash | sudo tee /etc/bash_completion.d/rx
_rx() {
    local cur words
    cur="${COMP_WORDS[COMP_CWORD]}"
    words="--repo --hostname --keep --jobs --cores --update --diff --yes \
--only --dry-run --no-color --completions --help --version \
-r -n -k -j -c -u -d -y -h -v"

    case "${COMP_WORDS[COMP_CWORD-1]}" in
        -r|--repo)
            COMPREPLY=( $(compgen -f -- "$cur") )
            return
            ;;
        --only)
            COMPREPLY=( $(compgen -W "pull flake stage rebuild cleanup diff" -- "$cur") )
            return
            ;;
        --completions)
            COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
            return
            ;;
    esac

    COMPREPLY=( $(compgen -W "$words" -- "$cur") )
}

complete -F _rx rx
"#;

const ZSH: &str = r#"#compdef rx
# zsh completion for rx
# Install: rx --completions zsh > "${fpath[1]}/_rx"
_rx() {
    _arguments -s \
        '(-r --repo)'{-r,--repo}'[path to the dotfiles repo]:repo:_files -/' \
        '(-n --hostname)'{-n,--hostname}'[hostname for the flake attribute]:hostname:' \
        '(-k --keep)'{-k,--keep}'[generations to keep]:count:' \
        '(-j --jobs)'{-j,--jobs}'[cap nix max-jobs]:count:' \
        '(-c --cores)'{-c,--cores}'[cap nix cores]:count:' \
        '(-u --update)'{-u,--update}'[update flake inputs first]' \
        '(-d --diff)'{-d,--diff}'[show the closure diff]' \
        '(-y --yes)'{-y,--yes}'[assume yes for all prompts]' \
        '--only[run only these steps]:steps:(pull flake stage rebuild cleanup diff)' \
        '--dry-run[print commands without executing them]' \
        '--no-color[disable colored output]' \
        '--completions[print shell completions]:shell:(bash zsh fish)' \
        '(-h --help)'{-h,--help}'[print help]' \
        '(-v --version)'{-v,--version}'[print the version]'
}

_rx "$@"
"#;

const FISH: &str = r#"# fish completion for rx
# Install: rx --completions fish > ~/.config/fish/completions/rx.fish
complete -c rx -s r -l repo -d 'path to the dotfiles repo' -r -F
complete -c rx -s n -l hostname -d 'hostname for the flake attribute' -r
complete -c rx -s k -l keep -d 'generations to keep' -r
complete -c rx -s j -l jobs -d 'cap nix max-jobs' -r
complete -c rx -s c -l cores -d 'cap nix cores' -r
complete -c rx -s u -l update -d 'update flake inputs first'
complete -c rx -s d -l diff -d 'show the closure diff'
complete -c rx -s y -l yes -d 'assume yes for all prompts'
complete -c rx -l only -d 'run only these steps' -r -f -a "pull flake stage rebuild cleanup diff"
complete -c rx -l dry-run -d 'print commands without executing them'
complete -c rx -l no-color -d 'disable colored output'
complete -c rx -l completions -d 'print shell completions' -r -f -a "bash zsh fish"
complete -c rx -s h -l help -d 'print help'
complete -c rx -s v -l version -d 'print the version'
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_names_round_trip() {
        for shell in Shell::ALL {
            assert_eq!(Shell::from_name(shell.name()), Some(shell));
            assert_eq!(shell.to_string(), shell.name());
        }
        assert!(Shell::from_name("powershell").is_none());
        assert_eq!(Shell::names(), "bash,zsh,fish");
    }

    #[test]
    fn every_script_completes_the_binary_and_its_flags() {
        for shell in Shell::ALL {
            let script = shell.script();
            assert!(script.contains("rx"));
            // Fish spells long options `-l only`.
            assert!(script.contains("only"));
            assert!(script.contains("completions"));
        }
    }

    #[test]
    fn print_writes_the_script() {
        let mut buf = Vec::new();
        assert!(print(&mut buf, Shell::Bash).is_ok());
        assert_eq!(String::from_utf8_lossy(&buf), BASH);
    }

    #[test]
    fn print_propagates_write_errors() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print(&mut writer, Shell::Fish).is_err());
    }
}
