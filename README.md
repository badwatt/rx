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
3. **Git Stage** — detects changes (incl. untracked files), asks confirmation, `git add .`
4. **NixOS Rebuild** — `nixos-rebuild switch --flake ...#{hostname}`. The installed
   `nixos-rebuild` is probed (`--elevate=none list-generations`): the `-ng`
   rewrite runs as your user with `--elevate=sudo --no-reexec`, the classic
   script uses `sudo ... --fast`. Either way the self-rebuild re-exec is skipped.
5. **Cleanup** — `nix-env --delete-generations +N` then `nix-collect-garbage`
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
| `--yes`        | `-y`  | Assume yes for all prompts                | `false`       |
| `--dry-run`    |       | Print commands without executing them     | `false`       |
| `--help`       | `-h`  | Print help                                | —             |
| `--version`    | `-v`  | Print version                             | —             |

`--keep` must be at least `1`. If the hostname cannot be detected, `rx` stops
before rebuilding; pass `--hostname` explicitly.

Hostnames resolve in order: `--hostname` flag → `/proc/sys/kernel/hostname` → `$HOSTNAME` env → `hostname` command → `"unknown"`.

---

## 🏗 Architecture

Hexagonal (ports & adapters) core organized into vertical slices. Top-level
directories scream the capabilities (`system_update`, `cli`); `domain` is the
shared pure kernel and `ports`/`adapters` hold the hexagonal boundaries.

```
src/
├── main.rs              # Composition root: parse → wire adapters → run
├── domain/              # Shared kernel (pure)
│   ├── config.rs        # Config entity + validation
│   └── error.rs         # Error type
├── ports/               # Semantic interfaces the core depends on
│   ├── repo.rs          # Repo: is_repo / pull / changes / add_all
│   ├── nix.rs           # Nix: update / rebuild / prune / collect_garbage / diff_closures
│   └── user_io.rs       # UserIo: title / confirm / error / success / output / config
├── adapters/            # Driven adapters
│   ├── command.rs       # CommandRunner (adapter-internal argv seam)
│   ├── process.rs       # ProcessRunner (std::process)
│   ├── dry_run.rs       # DryRunRunner (echo commands, --dry-run)
│   ├── git.rs           # GitRepo: Repo via the git CLI
│   ├── nix.rs           # NixCli: Nix via nix / nixos-rebuild
│   └── terminal.rs      # TerminalIo (stdin/stdout) + config rendering
├── system_update/       # Use-case slice
│   ├── mod.rs           # Pipeline orchestration
│   └── steps/           # pull, flake, stage, rebuild, cleanup, diff
├── cli/                 # Driving adapter
│   ├── mod.rs           # Intent
│   ├── parse.rs         # Argument parser
│   ├── defaults.rs      # Default config + tilde expansion
│   ├── hostname.rs      # Hostname resolution
│   └── help.rs          # Help/version banners
├── ansi.rs              # ANSI color helpers + write_flush
└── test_helpers.rs      # FailingWriter / StubRunner etc.
tests/
└── integration.rs        # End-to-end binary tests
```

### Design decisions

- **Zero dependencies** — no crates, pure `std`. Compile fast, audit easy.
- **Semantic ports** — the use case depends on `Repo`, `Nix` and `UserIo`, never on
  `git`/`nix` specifics. `adapters::git` / `adapters::nix` own the argv; the
  workflow tests mock the semantic ports. `--dry-run` decorates a single
  adapter-internal `CommandRunner` seam, so no per-port dry adapters.
- **Composition in `main`** — the driving adapter (`cli`) only parses; `main`
  selects adapters and wires them, so `cli` never depends on `adapters`.
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
240 tests: 228 unit + 12 integration — all passing ✅
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