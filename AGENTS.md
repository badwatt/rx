# AGENTS.md — Project Rules for `rx`

## Communication

- **CAVEMAN MODE FULL** always. Drop articles, filler, pleasantries. Short synonyms. Fragments fine. Technical terms exact. Code blocks unchanged. Pattern: thing action reason.
- Conversation in Spanish OK. Code, commits, docs in English.

## Workflow

- **TDD estricto**: write failing test first, then implement, then green. Always.
- **Coverage 100% strict**: regions, functions, lines — all must be 100%. Command:
  `cargo llvm-cov --all-targets --no-default-ignore-filename-regex`. No exceptions.
- **Never push without explicit permission**. Ask every time. No autonomous git push ever.
- Ask before destructive operations (purge, delete, force-push, rebase).
- **CI must stay green** (`.github/workflows/ci.yml`): `fmt`, `clippy -D warnings`, `test`, `coverage`.

## Coverage

- Tool: `cargo llvm-cov --all-targets --no-default-ignore-filename-regex` exclusively. Rejected alternatives (tarpaulin).
- **Tests count too**: `--no-default-ignore-filename-regex` disables cargo-llvm-cov's built-in exclusion of `tests.rs` / `tests/`. Without it test code is silently ignored.
- Toolchain pinned in `rust-toolchain.toml` (1.98). No nightly, no `#![feature(coverage_attribute)]` currently.
- **Prefer structural fixes** over `#[coverage(off)]`. Use it only as last resort for impossible-to-cover branches.
- **`dyn Write` over generics**: prefer `&mut dyn Write` instead of `<W: Write>` for IO functions. Generic monomorphizations create uncovered per-type regions.
- Every `?` creates a branch. Both paths (Ok, Err) must be exercised by tests.
- No `#[should_panic]` tests — they leave the normal return path uncovered. Use `std::panic::catch_unwind` in normal tests if panic-path coverage is needed.
- Closures are instrumented per expression site, not per call — share one named `fn` for common mock behavior so its branches merge across tests.

## Code Style

- **No `.unwrap()` anywhere** — neither production nor test code.
- Use `.expect("context")` with descriptive messages in tests.
- Use `.is_ok()`, `.is_err()`, `.is_some()`, `.is_none()` for boolean assertions.
- Use `String::from_utf8_lossy(&buf)` instead of `String::from_utf8(buf).unwrap()`.
- Use `.expect_err("context")` instead of `.unwrap_err()`.
- **Rust best practices skill**: always apply `rust-best-practices` skill guidelines (borrowing over cloning, `?` over match chains, iterators over loops, descriptive test names, one assertion per test when possible, doc comments on items).
- **SOLID / DRY / Clean Code**: extract functions for single responsibility. DRY repeated patterns. Keep functions small and readable.
- **Zero dependencies preference**: keep `Cargo.toml` dependency-free unless strong justification. Manual `Display`/`Error` impls over `thiserror` for binaries.
- **`pub(crate)`, not `pub`**: this is a binary crate; `pub` exposes a fictional API surface.

## Linting

- Run `cargo clippy --all-targets -- -D warnings` before considering work done. Zero warnings allowed.
- Use `#[expect(clippy::lint_name, reason = "...")]` over `#[allow(...)]` if a lint must be suppressed.
- Prefer `assert_ne!(x, "")` / `assert_eq!(x, "")` over `assert!(!x.is_empty())` — `clippy::assert_is_empty` (1.99) rejects the latter.
- The toolchain pin exists to stop new stable lints from breaking CI silently.

## Commits

- **Conventional Commits** format: `type(scope): description`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `style`, `chore`, `ci`, `perf`, `build`, `revert`
- Imperative mood in description. Body optional for context.
- Small, focused commits preferred.
- Generate message from `git diff --cached` before committing.

## Testing

- Descriptive test names: `unit_should_return_x_when_y` or `function_returns_error_when_input_empty`.
- One assertion per test when possible.
- No `assert!`/`assert_eq!` macro overrides for coverage — use structural fixes instead.
- Test helpers (e.g., `no_run`, `ok_ui`) must have both branches covered.
- Unit tests live next to their module: inline `#[cfg(test)] mod tests` or a sibling `tests.rs` (`#[cfg(test)] mod tests;`).
- Integration tests in `tests/integration.rs` — exercise the compiled binary via `Command`.
- **Integration tests must be side-effect free.** Never let them run the real workflow: use `--dry-run -y`, decline on stdin (`n`), or a nonexistent `--repo`. Otherwise they can `git pull`/`git add` in the user's real `~/.dotfiles`.
- When a test drives the rebuild step, pass `--hostname` explicitly — the Nix sandbox has no `hostname` command and detection yields `"unknown"`.

## Architecture

Hexagonal (ports & adapters) core organized into vertical slices.

```
src/
├── main.rs              # composition root: wires adapters, maps exit codes
├── config.rs            # Config entity (pure domain)
├── commands.rs          # argv specs for git/nix (shared vocabulary)
├── error.rs             # Error enum
├── ansi.rs              # ANSI color helpers + write_flush
├── ports/               # interfaces the core depends on
│   ├── command.rs       # CommandRunner: run(argv), capture(argv), probe(argv)
│   └── user_io.rs       # UserIo: title/confirm/error/success/output/config
├── adapters/            # driven adapters
│   ├── process.rs       # ProcessRunner (std::process)
│   ├── dry_run.rs       # DryRunRunner (echo commands, --dry-run)
│   └── terminal.rs      # TerminalIo (stdin/stdout) + config rendering
├── update/              # use-case slice: NixOS update workflow
│   ├── mod.rs           # pipeline orchestration
│   └── steps/           # pull, flake, stage, rebuild, cleanup, diff
└── cli/                 # driving adapter
    ├── mod.rs           # Intent + dispatch (selects DryRunRunner)
    ├── parse.rs         # argument parser
    ├── defaults.rs      # default config + tilde expansion
    ├── hostname.rs      # hostname resolution
    ├── nixos.rs         # NixOS release detection
    └── help.rs          # help/version banners
```

- **Ports**: `CommandRunner` (run/capture/probe), `UserIo`. Core depends only on these.
- **Single output channel**: all user-facing output flows through `UserIo`.
- **`commands.rs` at the crate root on purpose**: the use-case names the commands it runs; moving these specs into `adapters/` would invert the hexagonal dependency direction. They are pure data (no I/O).
- **Error enum**: separate variants per failure mode — `Io`, `GitPullFailed`, `GitStatusFailed`, `NotAGitRepo`, `FlakeUpdateFailed`, `RebuildFailed`, `HostnameUnknown`, `CleanupFailed`, `GitAddFailed`, `InvalidArgs`, `InvalidConfig`.
- **No shell**: every external command runs via argv. The closure diff is captured and filtered in Rust (no `tac`/`awk`).
- **Fail-fast**: every step checks the command exit status; a failure aborts before the next step, so cleanup never runs after a failed rebuild.
- **Capability-probed rebuild**: `rx` probes the installed `nixos-rebuild` via `CommandRunner::probe` (`nixos-rebuild --elevate=none list-generations`); exit 0 → `-ng` (`--elevate=sudo --no-reexec`), anything else → classic (`sudo … --fast`). Both skip the self-rebuild re-exec.
- **Validated config**: `Config::validate()` rejects empty repo/hostname or `keep == 0` with `Error::InvalidConfig` (exit code 2).
- **Cleanup**: `nix-env --delete-generations +N` then `nix-collect-garbage` (no `-d`, so `--keep N` is respected).

## Build / packaging

- `flake.nix`: `packages.default` / `packages.rx`; cross builds under `legacyPackages.<system>.cross.<target>.default`; devShell ships `cargo-llvm-cov` + LLVM (`LLVM_COV` / `LLVM_PROFDATA`).
- `nix build` (default) and `nix develop` must keep working.
- CI pins rust 1.98 via `dtolnay/rust-toolchain@1.98`; keep it in sync with `rust-toolchain.toml`.
