<div align="center">

<img src="assets/rx-banner.svg" alt="RX — NixOS Update CLI" width="800"/>

<br/>

### A minimal CLI to pull, stage, rebuild, and clean up your NixOS system — in one shot.

[![Rust](https://img.shields.io/badge/rust-2024-edition-000?style=flat-square&logo=rust&logoColor=DEA584)](https://doc.rust-lang.org/edition-guide/rust-2024/)
[![Tests](https://img.shields.io/badge/tests-161_passing-3FB950?style=flat-square)](./tests)
[![Coverage](https://img.shields.io/badge/coverage-100%25-3FB950?style=flat-square)](./src)
[![Dependencies](https://img.shields.io/badge/dependencies-zero-58A6FF?style=flat-square)](./Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](./LICENSE)

</div>

---

## ✨ What it does

`rx` automates the full NixOS update cycle:

1. **Git Pull** — pulls latest changes from your dotfiles repo
2. **Nix Flake Update** — `nix flake update` *(optional)*
3. **Git Stage** — detects diff, asks confirmation, `git add .`
4. **NixOS Rebuild** — `sudo nixos-rebuild switch --flake ...#{hostname}`
5. **Cleanup** — `nix-env --delete-generations +N`
6. **Diff** — shows closure diff between old/new generations *(optional)*

Every step — interactive confirmation, colored ANSI output, clean error handling.

---

## 🚀 Quick start

### Build

```bash
git clone https://github.com/user/rx.git
cd rx
cargo build --release
./target/release/rx
```

### Nix

```bash
nix build                          # default package
nix build .#default                # explicit attribute
nix build .#cross.x86_64-linux-musl.default --impure  # cross-compile
nix develop                        # dev shell
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
| `--update`    | `-u`  | Run `nix flake update` before rebuild    | `false`       |
| `--diff`       | `-d`  | Show closure diff after rebuild           | `false`       |
| `--help`       | `-h`  | Print help                                | —             |
| `--version`    | `-v`  | Print version                             | —             |

Hostnames resolve in order: `--hostname` flag → `/proc/sys/kernel/hostname` → `$HOSTNAME` env → `hostname` command → `"unknown"`.

---

## 🏗 Architecture

Hexagonal (ports & adapters) core organized into vertical slices. Top-level
directories scream the use cases (`update`, `cli`); the hexagonal layers
(`ports`, `adapters`) hold the shared boundaries.

```
src/
├── main.rs              # Composition root: wires adapters, maps exit codes
├── error.rs             # Error type (Io, GitPullFailed, InvalidArgs)
├── config.rs            # Config entity (pure domain)
├── commands.rs          # Command argv specs (git, nix)
├── ansi.rs              # ANSI color helpers + write_flush
├── test_helpers.rs      # FailingWriter / FailingFlushWriter etc.
├── ports/               # Interfaces the core depends on
│   ├── command.rs       # CommandRunner port (argv + shell pipelines)
│   └── user_io.rs       # UserIo port (title, confirm, output, config)
├── adapters/            # Driven adapter implementations
│   ├── process.rs       # ProcessRunner (std::process)
│   └── terminal.rs      # TerminalIo (stdin/stdout) + config rendering
├── update/              # Use-case slice: NixOS update workflow
│   ├── mod.rs           # Pipeline orchestration
│   └── steps/           # pull, flake, stage, rebuild, cleanup, diff
├── cli/                 # Driving adapter: command-line interface
│   ├── mod.rs           # Intent + dispatch
│   ├── parse.rs         # Argument parser
│   ├── defaults.rs      # Default config + tilde expansion
│   ├── hostname.rs      # Hostname resolution
│   └── help.rs          # Help/version banners
tests/
└── integration.rs        # End-to-end binary tests
```

### Design decisions

- **Zero dependencies** — no crates, pure `std`. Compile fast, audit easy.
- **Ports & adapters** — the workflow depends only on the `CommandRunner` and `UserIo`
  traits. Tests inject mocks; production wires `ProcessRunner` / `TerminalIo` in `main`.
- **Single output channel** — all user-facing output flows through the `UserIo` port.
- **Command builders return `Vec<String>`** — argv lists, not shell strings. No injection risk.
- **Shell pipelines only when necessary** — `nix_diff` is the sole `sh -c` call (requires `tac` + `awk`).
- **Clippy pedantic** — `#[warn(clippy::pedantic)]` enforced at compile time.

---

## 🧪 Testing

### 100% test coverage — every branch, every error path

```
164 tests: 154 unit + 10 integration — all passing ✅
```

Unit tests cover:
- Arg parsing (valid, invalid, edge cases, defaults)
- ANSI helpers (color codes, flush behavior, failing writers)
- Command builders (every `git_*` and `nix_*` function)
- Process execution (exit codes, signals, suppressed output)
- User IO (confirm prompt, parse responses, banners, config rendering, error propagation)
- Config (hostname resolution chain, tilde expansion)
- Update workflow (pull, flake, stage, rebuild, cleanup, diff — with mock ports)
- Error type (Display, Error::source, From impl)

Integration tests cover:
- Help/version flags (short and long)
- Unknown flags → exit code 2
- Piped stdin doesn't panic
- Subcommand words (`rx help`, `rx version`)

```bash
cargo test          # run all tests
cargo clippy        # lint
cargo fmt --check   # format check
```

---

## 🔧 Nix flake

```nix
{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";

  outputs = { self, nixpkgs, ... }:
    # Build: nix build .#default
    # Cross: nix build .#cross.x86_64-linux-musl.default --impure
    # Dev:   nix develop
}
```

Cross-compilation targets:
- `x86_64-linux` / `x86_64-linux-musl`
- `aarch64-linux` / `aarch64-linux-android`
- `x86_64-darwin` / `aarch64-darwin`

---

## 📜 License

MIT