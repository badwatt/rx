<div align="center">

<img src="assets/rx-banner.svg" alt="RX — NixOS Update CLI" width="800"/>

<br/>

### A minimal CLI to pull, stage, rebuild, and clean up your NixOS system — in one shot.

[![Rust](https://img.shields.io/badge/rust-2024-edition-000?style=flat-square&logo=rust&logoColor=DEA584)](https://doc.rust-lang.org/edition-guide/rust-2024/)
[![CI](https://github.com/badwatt/rx/actions/workflows/ci.yml/badge.svg)](https://github.com/badwatt/rx/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-100%25-3FB950?style=flat-square)](./src)
[![Dependencies](https://img.shields.io/badge/dependencies-zero-58A6FF?style=flat-square)](./Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](./LICENSE)

</div>

---

## ✨ What it does

`rx` automates the full NixOS update cycle:

1. **Git Pull** — pulls latest changes from your dotfiles repo
2. **Nix Flake Update** — `nix flake update` *(optional)*
3. **Git Stage** — detects changes (incl. untracked files), asks confirmation, `git add .`
4. **NixOS Rebuild** — `nixos-rebuild switch --flake ...#{hostname}`. The installed
   `nixos-rebuild` is probed (`--elevate=none list-generations`): the `-ng`
   rewrite runs as your user with `--elevate=sudo --no-reexec`, the classic
   script uses `sudo ... --fast`. Either way the self-rebuild re-exec is skipped.
5. **Cleanup** — `nix-env --delete-generations +N` then `nix store gc`
6. **Diff** — shows closure diff between old/new generations *(optional)*

Every step — interactive confirmation, colored ANSI output, clean error handling.

---

## 🚀 Quick start

### Build

```bash
git clone https://github.com/badwatt/rx.git
cd rx
cargo build --release
./target/release/rx
```

### Shell completions and man page

```bash
rx --completions bash | sudo tee /etc/bash_completion.d/rx   # zsh, fish too
man rx                                                        # installed by nix/cargo packaging
```

The completion scripts are static (no generator crate) and the man page is
shipped as `rx.1`; the flake installs it under `share/man/man1`.

### Release binary

Every `VERSION` bump on `main` gets published as a GitHub release with static
binaries for:

| Target                        | Runner        | Notes                     |
|-------------------------------|---------------|---------------------------|
| `x86_64-unknown-linux-musl`    | ubuntu-latest | static, no glibc          |
| `aarch64-unknown-linux-musl`   | ubuntu-latest | cross-built               |
| `aarch64-apple-darwin`         | macos-latest  | native, unsigned          |
| `x86_64-apple-darwin`          | macos-latest  | cross-built on arm64 host |

Each release also ships `SHA256SUMS`. The macOS binaries are unsigned, so
Gatekeeper quarantines them after download; clear it with
`xattr -d com.apple.quarantine ./rx`.

A manual `workflow_dispatch` run always builds the matrix (validating the
action versions and cross targets) and only publishes when the `publish` input
is enabled.

```bash
gh release download v0.6.0 --pattern '*x86_64-unknown-linux-musl*'
tar -xzf rx-v0.6.0-x86_64-unknown-linux-musl.tar.gz
./rx-v0.6.0-x86_64-unknown-linux-musl/rx --version
```

### Versioning

`VERSION` (repository root) is the single source of truth. `Cargo.toml` carries
a `0.0.0` placeholder and `flake.nix` reads the file. To release, bump
`VERSION`; CI builds, tags `v<VERSION>` and attaches the binaries. A push that
does not change `VERSION` publishes nothing.

Tradeoff: because `cargo` cannot read a version file, `Cargo.toml` keeps a
`0.0.0` placeholder. `cargo install --path .` therefore records `0.0.0` in the
cargo metadata, while `rx --version` always prints the real `VERSION`.

### Nix

```bash
nix build                          # default package
nix build .#default                # explicit attribute
nix build .#cross.x86_64-linux-musl.default --impure  # cross-compile
nix develop                        # dev shell
nix flake check                    # evaluate outputs + build the package
```

---

## ⚙️ Usage

```
$ rx

*********************
* RX Configuration  *
*********************

◉ repo     = ~/.dotfiles
◉ hostname = mymachine
◉ keep     = 10
◉ update   = false
◉ diff     = false

Proceed? (Y/n)
```

### Flags

| Flag           | Short | Description                              | Default       |
|----------------|-------|------------------------------------------|---------------|
| `--repo`       | `-r`  | Path to dotfiles repo                    | `~/.dotfiles` |
| `--hostname`   | `-n`  | Hostname for `nixos-rebuild --flake`     | auto-detected |
| `--keep`       | `-k`  | Generations to keep during cleanup       | `10`          |
| `--jobs`       | `-j`  | Cap Nix `--max-jobs` for the rebuild     | unset         |
| `--cores`      | `-c`  | Cap Nix `--cores` for the rebuild        | unset         |
| `--update`    | `-u`  | Run `nix flake update` before rebuild    | `false`       |
| `--diff`       | `-d`  | Show closure diff after rebuild           | `false`       |
| `--yes`        | `-y`  | Assume yes for all prompts                | `false`       |
| `--only`       |       | Run only these steps (comma separated)   | all           |
| `--dry-run`    |       | Print commands; only the read-only probe runs | `false`   |
| `--no-color`   |       | Disable colored output                    | auto          |
| `--completions`|       | Print shell completions (bash, zsh, fish) | —             |
| `--help`       | `-h`  | Print help                                | —             |
| `--version`    | `-v`  | Print version                             | —             |

`--keep` must be at least `1` (up to `255`); `--jobs`/`--cores` go up to `65535`.
Out-of-range values name the accepted range. If the hostname cannot be
detected, `rx` stops before touching the repository; pass `--hostname`
explicitly.

Hostnames resolve in order: `--hostname` flag → `/proc/sys/kernel/hostname` →
`$HOSTNAME` env → `hostname` command. When every source fails the summary shows
`<not detected>` and the run aborts with a hint.

Long flags also accept `--flag=value` (`--repo=/path`, `--only=diff`).

### Step selection

`--only` takes a comma-separated list of `pull`, `flake`, `stage`, `rebuild`,
`cleanup`, `diff` (`update` is accepted as an alias for `flake`):

```bash
rx --only diff                 # only show the closure diff
rx -u --only flake            # just update flake inputs
rx --only stage,rebuild       # no pull, no cleanup
```

An explicit list wins over `--update`/`--diff`, so the printed summary always
matches what runs. Declining the staging prompt still aborts the run.

---

## 🏗 Architecture

Feature-first hexagonal: every feature screams its capability (`system_update`,
`cli`) and owns its layers (`application` / `ports` / `adapters`). Only the
domain kernel, non-business helpers and generic infrastructure are shared.

```
src/
├── main.rs              # Composition root: parse → wire adapters → run
├── domain/              # Shared kernel (pure)
│   ├── config.rs        # Config + Step + validate() + BuildOptions
│   └── error.rs         # Error type
├── shared/              # Non-business cross-cutting
│   ├── ansi.rs          # ANSI codes, Palette (NO_COLOR/TTY), write_flush
│   └── test_helpers.rs  # FailingWriter / FailAfterWriter / StubRunner
├── infra/               # Generic process mechanism (not business, not a port)
│   ├── command.rs       # CommandRunner (adapter-internal argv seam)
│   ├── process.rs       # ProcessRunner (std::process)
│   └── dry_run.rs       # DryRunRunner (echo commands, --dry-run)
├── system_update/       # Feature: the NixOS update workflow
│   ├── application/     # Use cases
│   │   ├── mod.rs       # Pipeline orchestration
│   │   └── steps/       # pull, flake, stage, rebuild, cleanup, diff
│   ├── ports/           # Interfaces the feature needs
│   │   ├── repo.rs      # Repo: is_repo / pull / changes / add_all
│   │   ├── nix.rs       # Nix: update / rebuild / prune / collect_garbage / diff_closures
│   │   └── user_io.rs   # UserIo: title / confirm / success / output / config
│   └── adapters/        # Driven implementations for the feature
│       ├── git.rs       # GitRepo: Repo via the git CLI
│       ├── nix.rs       # NixCli: Nix via nix / nixos-rebuild
│       └── terminal.rs  # TerminalIo (stdout/stderr) + config rendering
├── cli/                 # Driving adapter
│   ├── mod.rs           # Intent
│   ├── parse.rs         # Argument parser
│   ├── defaults.rs      # Default config + tilde expansion
│   ├── hostname.rs      # Hostname resolution
│   └── help.rs          # Help/version banners
tests/
└── integration.rs        # End-to-end binary tests
```

### Design decisions

- **Zero dependencies** — no crates, pure `std`. Compile fast, audit easy.
- **Semantic ports** — the use case depends on `Repo`, `Nix` and `UserIo`, never on
  `git`/`nix` specifics. `system_update::adapters::git` / `::nix` own the argv; the
  workflow tests mock the semantic ports. `--dry-run` decorates a single
  adapter-internal `CommandRunner` seam, so no per-port dry adapters.
- **Composition in `main`** — the driving adapter (`cli`) only parses; `main`
  selects adapters and wires them, so `cli` never depends on feature adapters.
- **Diagnostics on stderr, once** — parse errors and the single top-level error
  go to stderr; titles, prompts and status stay on stdout, so `rx | head` keeps
  errors visible and no failure is printed twice.
- **NO_COLOR / non-TTY / `--no-color`** — each stream picks its own `Palette`
  from `NO_COLOR` (when non-empty) and `is_terminal()`; `--no-color` forces
  plain output. A closed stdin aborts prompts instead of answering "yes".
- **Exit codes** — `2` for invalid arguments/config, `141` (128 + SIGPIPE) when
  stdout closes early, `128 + N` for signal-terminated child commands, else `1`.
- **Parallelism caps** — `-j/--jobs` and `-c/--cores` map to `--max-jobs`/`--cores`,
  so a heavy build cannot OOM the terminal's systemd scope.
- **Single output channel** — all user-facing output flows through the `UserIo` port.
- **No shell** — every external command runs via argv; the closure diff is captured and filtered in Rust (no `tac`/`awk`).
- **Capability-probed rebuild** — `rx` probes the installed `nixos-rebuild`
  (`--elevate=none list-generations`, read-only) instead of trusting the OS
  version; `-ng` gets `--elevate=sudo --no-reexec`, classic gets `sudo --fast`.
- **Validated config** — `Config::validate()` rejects an empty repo/hostname or
  `keep == 0` up front with a clear message.
- **Fail-fast** — every step checks the command exit status; a failed pull,
  flake update, rebuild or cleanup aborts before the next step (so cleanup never
  runs after a failed rebuild).
- **Clippy pedantic** — `#[warn(clippy::pedantic)]` enforced at compile time.

---

## 🧪 Testing

### 100% coverage — every branch, every error path

```
288 tests: 270 unit + 18 integration — all passing ✅
```

Coverage counts tests too: `--no-default-ignore-filename-regex` disables
cargo-llvm-cov's built-in exclusion of `tests.rs` / `tests/`.

Unit tests cover:
- Arg parsing (valid, invalid, edge cases, defaults, `--keep 0` guard)
- ANSI helpers (color codes, flush behavior, failing writers)
- Command builders (every `git_*` and `nix_*` function)
- Process execution (exit codes, signals, capture)
- User IO (confirm prompt, parse responses, banners, config rendering, error propagation)
- Config (hostname resolution chain, NixOS release detection, tilde expansion)
- Update workflow (pull, flake, stage, rebuild, cleanup, diff — with mock ports)
- Dry-run runner (command echoing)
- Error type (Display, Error::source, From impl)

Integration tests cover:
- Help/version flags (short and long)
- Unknown flags → exit code 2
- `--keep 0` → exit code 2
- `--dry-run` prints commands without executing
- Non-git `--repo` → non-zero exit
- Piped stdin `n` declines and exits 0
- Subcommand words (`rx help`, `rx version`)

```bash
cargo test          # run all tests
cargo clippy        # lint
cargo fmt --check   # format check
cargo llvm-cov --all-targets --no-default-ignore-filename-regex   # 100% (incl. tests)
```

---

## 🔧 Nix flake

```nix
{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";

  outputs = { self, nixpkgs, ... }:
    # Build: nix build        (or nix build .#rx / .#default)
    # Run:   nix run .#default -- --version
    # Cross: nix build .#cross.x86_64-linux-musl.default --impure
    # Dev:   nix develop       (includes cargo-llvm-cov + LLVM tools)
}
```

Cross-compilation targets:
- `x86_64-linux` / `x86_64-linux-musl`
- `aarch64-linux`
- `x86_64-darwin` / `aarch64-darwin`

### Add as a flake input

```nix
{
  inputs.rx.url = "github:badwatt/rx";
}
```

```nix
{ inputs, pkgs, ... }:
{
  home.packages = [ inputs.rx.packages.${pkgs.system}.default ];
}
```

---

## 📜 License

MIT