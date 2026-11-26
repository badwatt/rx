<div align="center">

<img src="assets/rx-banner.svg" alt="rx — NixOS update CLI" width="720"/>

### Pull, stage, rebuild and clean up your NixOS system — in one command.

[![Release](https://img.shields.io/github/v/release/badwatt/rx?style=flat-square)](https://github.com/badwatt/rx/releases/latest)
[![CI](https://github.com/badwatt/rx/actions/workflows/ci.yml/badge.svg)](https://github.com/badwatt/rx/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-100%25-3FB950?style=flat-square)](https://github.com/badwatt/rx/actions/workflows/ci.yml)
[![Dependencies](https://img.shields.io/badge/dependencies-zero-58A6FF?style=flat-square)](Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

`rx` runs the whole NixOS update cycle from your dotfiles repo: pull, flake
update, stage, `nixos-rebuild switch`, prune old generations, and show the
closure diff. It is a small single binary with **zero Rust dependencies**, a prompt before
anything destructive, and a `--dry-run` that prints every command it would run
(the released builds are static).

Built for one person and their laptop. Needs NixOS, `git`, `nix`, and `sudo`.

## Install

```bash
# straight from the flake
nix run github:badwatt/rx -- --version

# or build it
nix build github:badwatt/rx
./result/bin/rx

# or with cargo (Rust 1.100, the pinned toolchain)
cargo install --git https://github.com/badwatt/rx
```

Prebuilt static binaries (Linux x86_64/aarch64 musl, macOS x86_64/aarch64) and
`SHA256SUMS` are attached to every [release](https://github.com/badwatt/rx/releases/latest):

```bash
gh release download v26.11.4 --pattern '*x86_64-unknown-linux-musl*'
tar -xzf rx-v26.11.4-x86_64-unknown-linux-musl.tar.gz
./rx-v26.11.4-x86_64-unknown-linux-musl/rx --version
```

As a flake input:

```nix
{
  inputs.rx.url = "github:badwatt/rx";
}
```

```nix
{ inputs, pkgs, ... }: {
  environment.systemPackages = [ inputs.rx.packages.${pkgs.system}.default ];
}
```

The flake also installs the man page (`man rx`) and completions for bash, zsh
and fish.

## Usage

```console
$ rx

********************
* RX Configuration *
********************

◉ repo = ~/.dotfiles
◉ hostname = mymachine
◉ keep = 10
◉ update = false
◉ diff = false

Proceed? (Y/n)
```

```bash
rx                   # pull → stage → rebuild → cleanup
rx -u                # also update the flake inputs first
rx -d                # also show the closure diff at the end
rx --only diff       # nothing but the closure diff
rx --dry-run -y      # print every command, run none
```

| Flag            | Short | What it does                                      | Default       |
|-----------------|-------|---------------------------------------------------|---------------|
| `--repo`        | `-r`  | Path to the dotfiles repository                   | `~/.dotfiles` |
| `--hostname`    | `-n`  | Flake attribute to rebuild (`repo#hostname`)      | detected      |
| `--keep`        | `-k`  | Generations kept by the cleanup                   | `10`          |
| `--jobs`        | `-j`  | Cap `nix --max-jobs` for the rebuild              | unset         |
| `--cores`       | `-c`  | Cap `nix --cores` for the rebuild                 | unset         |
| `--update`      | `-u`  | Run `nix flake update` before staging             | `false`       |
| `--diff`        | `-d`  | Show the closure diff after the rebuild           | `false`       |
| `--yes`         | `-y`  | Assume yes for every prompt                       | `false`       |
| `--only`        |       | Run only the listed steps                         | default steps |
| `--dry-run`     |       | Print commands instead of executing them          | `false`       |
| `--no-color`    |       | Disable ANSI colours                              | auto          |
| `--completions` |       | Print a completion script (`bash`/`zsh`/`fish`)   | —             |
| `--help`        | `-h`  | Print the help banner                             | —             |
| `--version`     | `-v`  | Print the version                                 | —             |

`--keep` accepts `1–255`, `--jobs`/`--cores` accept `1–65535`; out-of-range
values name the accepted range. Long flags also take `--flag=value`
(`--repo=/path`, `--only=diff`).

`--only` takes a comma-separated subset of `pull`, `flake`, `stage`,
`rebuild`, `cleanup`, `diff` (`update` is an alias for `flake`). The list wins
over `--update`/`--diff`, so the summary always matches what runs.

## What it runs

| Step | Command | When |
|------|---------|------|
| **Pull** | `git -C <repo> pull --ff-only` | default |
| **Flake** | `nix flake update --flake <repo>` | `--update` / `--only flake` |
| **Stage** | `git -C <repo> status --porcelain`, then `git add .` after a prompt | default |
| **Rebuild** | `nixos-rebuild switch --flake <repo>#<host>` | default |
| **Cleanup** | `nix-env --delete-generations +N`, then `nix store gc` | default |
| **Diff** | `nix profile diff-closures` | `--diff` / `--only diff` |

The installed `nixos-rebuild` is probed with a read-only call
(`--elevate=none list-generations`): the `-ng` rewrite runs as your user with
`--elevate=sudo --no-reexec`, the classic script as `sudo … --fast`. Either way
the self-rebuild re-exec is skipped.

## Behaviour worth knowing

- **Prompts are real prompts.** Declining the first one prints `Aborted.` and
  exits `0`. Declining the staging prompt aborts before the rebuild, so a stale
  flake is never built. A closed stdin (`rx </dev/null`) aborts; it is never a
  yes.
- **Exit codes.** `2` for bad arguments or config, `141` (128 + SIGPIPE) when
  stdout closes early, `128 + N` when a child dies from signal `N`, else `1`.
- **Hostname** resolves as `--hostname` → `/proc/sys/kernel/hostname` →
  `$HOSTNAME` → `hostname`, trimming each source. If every source fails the
  summary shows `<not detected>` and the run stops before touching the repo.
- **sudo is asked once, up front.** `rx` checks whether a password is needed,
  runs `sudo -v` while the tab says *blocked*, and keeps the ticket alive during
  the build so a long rebuild does not stop halfway asking again.
- **Terminal status.** On a TTY, `rx` shows the terminal's OSC 9;4 progress pulse
  in the tab while it works (re-reported ten times a second, so it keeps moving
  and never goes stale) and hides it whenever a prompt needs you; it also emits
  OSC 7501 reports
  (`working`, `blocked`, `done`, `error`, `idle`), so supporting terminals can
  badge the tab too. `TERM=dumb` turns both off.
- **The repo must be the work-tree root.** A subdirectory of another
  repository is rejected, so `pull`/`add` never touch the enclosing repo.
- **`NO_COLOR`, non-TTY and `--no-color`** each disable colour per stream.

> [!TIP]
> `--dry-run` executes nothing except the read-only `nixos-rebuild` probe. It
> never calls `sudo`, so a preview cannot ask for a password or refresh a ticket.

## Completions and man page

```bash
rx --completions bash | sudo tee /etc/bash_completion.d/rx   # also zsh, fish
```

The scripts are static, so there is no generator crate and no build step. The
flake installs them together with `rx.1` under `share/`; `cargo install`
installs only the binary.

## Development

```bash
nix develop        # the pinned toolchain (rust-toolchain.toml), cargo-llvm-cov, actionlint
nix build          # packages.default (same as packages.rx)
nix flake check    # build the package and evaluate the outputs
```

Before a commit — and what CI enforces:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo llvm-cov --all-targets --no-default-ignore-filename-regex   # 100 %, tests included
```

`--no-default-ignore-filename-regex` matters: without it cargo-llvm-cov
silently skips `tests.rs` and `tests/`, so test code would not count.

Tests are unit tests next to the code plus an integration suite that drives the
real binary with `--dry-run`, a declined prompt or a nonexistent repo, so it
never touches your system. `nix flake check` gates the flake; `actionlint` (in the
devShell) checks the workflows before a push.

## Releasing

[CalVer](https://calver.org) in the [memos](https://github.com/usememos/memos)
style — `YY.MM.N`, e.g. `26.11.2`: the release month plus a counter for further
releases in that month.

`[package].version` in `Cargo.toml` is the single source of truth. The binary
embeds it with `env!("CARGO_PKG_VERSION")`, the flake reads it with
`builtins.fromTOML`, and the release workflow tags `v<version>`. Bump that
version and push; a push that does not change it publishes nothing, and a tag
that already exists is a green no-op.

Targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` and both
Apple Darwin targets.

Release notes are a categorized changelog built from the conventional commit
subjects since the previous tag (breaking, feat, fix, perf, refactor, test,
docs, style, build, ci, chore, revert) with a tag-to-tag diff link.

> [!WARNING]
> The macOS binaries are unsigned, so Gatekeeper quarantines them. After
> downloading: `xattr -d com.apple.quarantine ./rx`.

## Architecture

Feature-first hexagonal: the update workflow owns its layers, the domain kernel
stays pure, and nothing knows about `git`/`nix` outside the adapters.

```
src/main.rs              composition: parse → wire adapters → run
src/domain/              pure kernel: config, error, step
src/terminal/            ansi (palette), osc (7501), progress (9;4)
src/test_support.rs      test doubles, cfg(test)
src/infra/               process runner, dry-run decorator, exit helpers
src/system_update/
├── application/         the workflow, the elevated helper, the steps
├── ports/               Repo · Nix · UserIo (traits)
└── adapters/            git · nix (argv) · terminal
src/cli/                 parser, defaults, hostname, completions, help
tests/integration.rs     the real binary, side-effect free
```

- **Zero dependencies.** Pure `std`: fast builds, no supply chain, manual
  `Display`/`Error` impls.
- **Semantic ports.** The workflow depends on `Repo`, `Nix` and `UserIo`, never
  on `git`/`nix` details; `--dry-run` decorates one adapter-internal command
  seam instead of every port.
- **Composition in `main`.** `cli` only parses; `main` picks adapters, so the
  parser never depends on a feature adapter.
- **One error voice.** User output goes through `UserIo` on stdout; parse errors
  and the single top-level error go to stderr, printed once.
- **Fail-fast.** Every step checks its exit status; a failed rebuild never runs
  the cleanup.
- **No shell.** Every command runs via argv; the diff is filtered in Rust.

MIT licensed — see [LICENSE](LICENSE).
