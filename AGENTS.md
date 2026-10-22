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
- Toolchain pinned in `rust-toolchain.toml` (1.99). No nightly, no `#![feature(coverage_attribute)]` currently. No `rust-version` field: the pinned toolchain is the supported one, so there is no unverified MSRV claim.
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

Feature-first hexagonal: every feature screams its capability (`system_update`,
`cli`) and owns its layers (`application` / `ports` / `adapters`). Only the
domain kernel, non-business helpers and generic infrastructure are shared.

```
src/
├── main.rs              # composition root: parse → wire adapters → run
├── domain/              # shared kernel (pure): config.rs, error.rs
├── shared/              # non-business cross-cutting: ansi.rs, test_helpers.rs
├── infra/               # generic process mechanism (not business, not a port)
│   ├── command.rs       # CommandRunner (adapter-internal argv seam)
│   ├── process.rs       # ProcessRunner (std::process)
│   └── dry_run.rs       # DryRunRunner (echo commands, --dry-run)
├── system_update/       # feature: the NixOS update workflow
│   ├── application/     # use cases
│   │   ├── mod.rs       # pipeline orchestration
│   │   └── steps/       # pull, flake, stage, rebuild, cleanup, diff
│   ├── ports/           # the interfaces the feature needs
│   │   ├── repo.rs      # Repo: is_repo / pull / changes / add_all
│   │   ├── nix.rs       # Nix: update / rebuild / prune / collect_garbage / diff_closures
│   │   └── user_io.rs   # UserIo: title/confirm/success/output/config
│   └── adapters/        # driven implementations for the feature
│       ├── git.rs       # GitRepo: Repo via the git CLI
│       ├── nix.rs       # NixCli: Nix via nix / nixos-rebuild
│       └── terminal.rs  # TerminalIo (stdout/stderr) + config rendering
├── cli/                 # driving adapter (parsing only)
│   ├── mod.rs           # Intent
│   ├── parse.rs         # argument parser
│   ├── defaults.rs      # default config + tilde expansion
│   ├── hostname.rs      # hostname resolution
│   └── help.rs          # help/version banners
```

- **Feature-first layout**: the business capability owns `application` / `ports` / `adapters`, so a new feature touches one folder. `domain`, `shared` and `infra` stay top-level because they are cross-feature by nature.
- **Semantic ports**: the use case depends on `Repo`, `Nix`, `UserIo` — never on `git`/`nix` specifics. `system_update::adapters::git` / `::nix` own the argv; workflow tests mock the semantic ports.
- **`CommandRunner` is not a port**: it is an adapter-internal seam (in `infra`) so `--dry-run` decorates a single implementation instead of every port.
- **Composition in `main`**: `cli` only parses (`Intent`), `main` selects adapters (dry-run vs process) and wires them. `cli` must not depend on feature adapters.
- **Single output channel**: user-facing output flows through `UserIo`; parse errors and the single top-level error go to **stderr**, the rest to stdout.
- **NO_COLOR / non-TTY / `--no-color`**: each stream picks its own `ansi::Palette` from `NO_COLOR` (non-empty) + `is_terminal()`; `--no-color` forces plain.
- **Exit codes**: `2` invalid args/config, `141` (128 + SIGPIPE) on early stdout close, `128 + N` for signal-terminated children, else `1`.
- **Error enum**: separate variants per failure mode — `Io`, `GitPullFailed`, `GitStatusFailed`, `NotAGitRepo`, `FlakeUpdateFailed`, `RebuildFailed`, `HostnameUnknown`, `CleanupFailed`, `GitAddFailed`, `DiffFailed`, `InvalidArgs`, `InvalidConfig`.
- **No shell**: every external command runs via argv. The closure diff is captured and filtered in Rust (no `tac`/`awk`).
- **Fail-fast**: every step checks the command exit status; a failure aborts before the next step, so cleanup never runs after a failed rebuild.
- **Capability-probed rebuild**: `rx` probes the installed `nixos-rebuild` via `CommandRunner::probe` (`nixos-rebuild --elevate=none list-generations`); exit 0 → `-ng` (`--elevate=sudo --no-reexec`), anything else → classic (`sudo … --fast`). Both skip the self-rebuild re-exec.
- **Validated config**: `Config::validate()` rejects an empty repo, an explicitly empty hostname, `keep == 0`, or `jobs`/`cores == 0` with `Error::InvalidConfig` (exit code 2). An undetected hostname (`None`) is valid here; the workflow turns it into `Error::HostnameUnknown` before any side effect.
- **Prompt decline**: declining the initial prompt prints `Aborted.` and exits 0; declining staging aborts before the rebuild (no stale flake). A closed stdin (EOF) is never a "yes": it aborts, so an empty pipe cannot rebuild the system.
- **Step selection**: `--only pull,flake,stage,rebuild,cleanup,diff` runs an explicit subset; `Config::runs(Step)` owns the decision and a list wins over `--update`/`--diff`. Long flags also accept `--flag=value`.
- **Single error voice**: steps return `Error`; only `main` reports it (on stderr). The `UserIo` port has no `error()` so a failure can never print twice.
- **Non-interactive git**: `git pull --ff-only` with `GIT_TERMINAL_PROMPT=0` via `CommandRunner::run_with_env`, so a diverged branch or missing credentials fails instead of hanging.
- **Repository root required**: `Repo::is_repo` checks that `git rev-parse --show-prefix` is empty, so a subdirectory of another repository is rejected instead of pulling/staging the enclosing repo.
- **Dry-run honesty**: `DryRunRunner` delegates `probe` to the real runner (read-only, silent), so the preview shows the `nixos-rebuild` flavor actually installed.
- **Cleanup**: `nix-env --delete-generations +N` then `nix store gc` (no `-d`, so `--keep N` is respected).

## Build / packaging

- `flake.nix`: `packages.default` / `packages.rx` (with `meta.mainProgram = "rx"`, so `nix run` works); cross builds under `legacyPackages.<system>.cross.<target>.default`; devShell ships `cargo-llvm-cov` + LLVM (`LLVM_COV` / `LLVM_PROFDATA`).
- `nix build` (default) and `nix develop` must keep working.
- Release matrix (`.github/workflows/release.yml`): `x86_64-unknown-linux-musl` (tested + built natively), `aarch64-unknown-linux-musl` (via `cross`), `aarch64-apple-darwin` and `x86_64-apple-darwin` (macOS runners, unsigned). Artifacts are uploaded per target, checksummed (`SHA256SUMS`) and published by a separate job.
- A `workflow_dispatch` run always builds the matrix but only publishes with the `publish` input enabled and no tag for `VERSION`; use it to validate action bumps without a release.
- CI pins rust 1.99 via `dtolnay/rust-toolchain@1.99`; keep it in sync with `rust-toolchain.toml`.

## Versioning & releases

- `VERSION` (repository root) is the **single source of truth** for the project version. `Cargo.toml` carries a `0.0.0` placeholder (`cargo` cannot read the file), `flake.nix` reads `VERSION`, and the binary embeds it via `include_str!("../../VERSION")`.
- To release: bump `VERSION` and push to `main`. `.github/workflows/release.yml` then tests, builds a static `x86_64-unknown-linux-musl` binary, tags `v<VERSION>` and attaches `rx-v<VERSION>-x86_64-unknown-linux-musl.tar.gz`.
- The release job is idempotent: if the tag `v<VERSION>` already exists it publishes nothing, so unrelated pushes stay quiet.
- Never hardcode the version in code or tests — read `VERSION` (unit: `cli::help::version()`, integration: `version()` helper).
