# rx

**rx** — CLI mínimo para actualizar NixOS desde tu repo de dotfiles: `git pull`, `nix flake update`, stage de los cambios, `nixos-rebuild switch`, limpieza de generaciones y diff de closures. Cero dependencias de Rust; pensado para un único usuario en su máquina.

> ⚠️ **Usa las skills de Rust: obligatorio, sin excepciones.** Antes de escribir, revisar, refactorizar o portar código, carga las skills de Rust disponibles (descúbrelas si hace falta). Si hay varias, aplica la que corresponda; si dudas, todas. Código que contradiga una skill de Rust está mal aunque compile.

## 0. Comunicación

- Conversación en español; código, commits, docs y comentarios en inglés.
- CAVEMAN MODE: frases cortas, sin relleno. Términos técnicos exactos; los bloques de código no se tocan.
- Antes de implementar: di los supuestos. Si hay dos lecturas, preséntalas. Si algo no cuadra, para y pregunta.

## 1. Datos del proyecto: se leen, no se escriben

| Dato | Dónde vive | Cómo se lee |
|---|---|---|
| Nombre | `[package] name` (`rx`) | `env!("CARGO_PKG_NAME")` |
| Versión | `[package] version`, CalVer `YY.MM.N` | `env!("CARGO_PKG_VERSION")` |
| Edición | `[package] edition` (`2024`) | `Cargo.toml`, y el CI lo compila |
| MSRV | `rust-toolchain.toml` (1.99, pineado) | el devShell del flake |
| Tipo | binario (`src/main.rs`, `pub(crate)` en todo) | lo que exista en el repo |
| Toolchain y herramientas | `flake.nix` + `rust-toolchain.toml` | el devShell del flake |
| Cobertura mínima | `.github/workflows/ci.yml` | **100** regiones/funciones/líneas |

- No hay campo `rust-version`: el toolchain pineado **es** el soportado; no se declara un MSRV sin verificar.
- ⛔ Archivo `VERSION`, literales repetidos (README, rustdoc, `--version`, títulos) y tags escritos a mano.
- ⛔ Copiar aquí o en el README la edición, el MSRV o la lista de herramientas: se leen del repo.
- El workflow de release lee `[package] version` con un `awk` anclado a la tabla (`rx` no es workspace). Si algún día hay workspace (`version.workspace = true`), pasa a `cargo metadata --no-deps --format-version 1` / `cargo pkgid`. ⛔ Regex sobre el TOML que asuma `version = "..."` en cualquier sitio.
- El tag `v<version>` es **consecuencia** de la versión, nunca su causa.
- `name`, `version`, `edition` y `rust-version` se consultan con el usuario.

## 2. Arquitectura

- **Hexagonal** — dominio puro en el centro; `std::process`, `git`/`nix` solo en adaptadores. El dominio no conoce la herramienta.
- **Feature-first / vertical slicing** — cada feature es una carpeta autocontenida con `application`, `ports`, `adapters` y tests.
- **Screaming architecture** — los directorios dicen *qué hace* el sistema, no *qué herramienta usa*.
- **Clean code** — funciones cortas, un nivel de abstracción por función, nombres que no necesiten comentario.
- Dependencias hacia dentro: `adapters → ports → domain`. Nunca al revés.

```
src/main.rs                composición: parse → adaptadores → workflow
src/domain/                kernel puro: config.rs, error.rs, step.rs
src/shared/                ansi.rs, osc.rs, test_helpers.rs (cfg(test))
src/infra/                 command.rs (seam), process.rs, dry_run.rs, argv.rs
src/system_update/         feature
├── application/           run + steps/{pull,flake,stage,rebuild,cleanup,diff}.rs
├── ports/                 repo.rs · nix.rs · user_io.rs (traits)
└── adapters/              git.rs · nix/{mod,argv}.rs · terminal.rs
src/cli/                   parse, defaults, hostname, completions, help, entry
tests/integration.rs       binario real, sin efectos
```

### Invariantes de rx

- Puertos **semánticos** (`Repo`, `Nix`, `UserIo`); los adaptadores son dueños del argv. `CommandRunner` **no** es un puerto: es un seam interno de adaptador para que `--dry-run` decore una sola implementación.
- Composición en `main`; `cli` solo parsea (`Intent`) y nunca importa adaptadores del feature.
- **Un solo canal de salida**: todo lo del usuario va por `UserIo`; los errores de parseo y el único error de nivel superior van a **stderr**.
- `NO_COLOR` (no vacío) + `is_terminal()` por stream deciden la `Palette`; `--no-color` fuerza plano en ambos.
- **Exit codes**: `2` args/config inválidos, `141` (128 + SIGPIPE) al cerrar stdout, `128 + N` si el hijo muere por señal, si no `1`.
- **Error enum**: una variante por modo de fallo — `Io`, `GitPullFailed`, `GitStatusFailed`, `NotAGitRepo`, `FlakeUpdateFailed`, `RebuildFailed`, `PasswordFailed`, `HostnameUnknown`, `CleanupFailed`, `GitAddFailed`, `DiffFailed`, `InvalidArgs`, `InvalidConfig`.
- **Sin shell**: todo comando externo por argv. El filtrado del diff se hace en Rust.
- **Fail-fast**: cada paso comprueba el exit status; un fallo aborta antes del siguiente, así el cleanup nunca corre tras un rebuild fallido.
- **Rebuild probado por capacidad**: probe `nixos-rebuild --elevate=none list-generations`; exit 0 → `-ng` (`--elevate=sudo --no-reexec`), otro → clásico (`sudo … --fast`). Ambos evitan el re-exec.
- **Config validada** (`Config::validate` → exit 2): repo vacío, hostname explícitamente vacío, `keep == 0`, `jobs`/`cores == 0`. Un hostname no detectado (`None`) es válido aquí; el workflow lo convierte en `HostnameUnknown` antes de cualquier efecto.
- **Declinar / EOF**: declinar el prompt imprime `Aborted.` y sale 0; un stdin cerrado **nunca** es un sí. Declinar el staging aborta antes del rebuild.
- **Selección de pasos**: `--only pull,flake,stage,rebuild,cleanup,diff`; la lista gana sobre `--update`/`--diff`. Las flags largas aceptan `--flag=value`.
- **Una sola voz de error**: `UserIo` no tiene `error()`; solo `main` reporta.
- **Git no interactivo**: `git pull --ff-only` con `GIT_TERMINAL_PROMPT=0`, así una rama divergida o sin credenciales falla en vez de colgarse.
- **Raíz del repo obligatoria**: `git rev-parse --show-prefix` vacío; un subdirectorio de otro repo se rechaza.
- **Hostname**: `/proc/sys/kernel/hostname` → `$HOSTNAME` → `hostname` (cada fuente se recorta; una vacía pasa a la siguiente).
- **Sudo**: `sudo -n true` (probe silencioso) y `sudo -v` (prompt) antes del comando elevado, nunca en `--dry-run`; un hilo detached renueva el ticket con `sudo -n -v` cada 60 s para que un build largo no vuelva a preguntar sin anunciarlo.
- **OSC 7501**: `UserIo::progress` reporta `working`, `blocked:kind=permission|auth`, `done` y `error` solo en TTY y nunca en `--dry-run`; tras cada prompt se vuelve a `working` o el tab sin foco se queda en "needs attention".
- **Completions**: `--completions bash|zsh|fish` imprime scripts estáticos; un test garantiza que mencionan el binario y las flags compartidas. `rx.1` es la página de manual.
- **Dry-run honesto**: `DryRunRunner` delega `probe` al runner real (silencioso), así el preview muestra el flavor realmente instalado; el resto no se ejecuta y sudo no se toca.
- **Cleanup**: `nix-env --delete-generations +N` y luego `nix store gc` (sin `-d`, para respetar `--keep N`).

## 3. Reglas

| ⛔ Nunca | ✅ En su lugar |
|---|---|
| `unsafe`, punteros raw, `transmute`, `mem::forget`, `ManuallyDrop`, `MaybeUninit`, `Box::leak` | Cero tolerancia, sin excepciones |
| `Rc`/`RefCell` fuera de tests | Préstamos o propiedad única; tests pueden usar `Rc` para registrar llamadas |
| `Arc` por reflejo · `clone()` para calmar al borrow checker | Préstamos (`&T`, `&str`, `&[T]`), mover valores, `Box<T>` si el dueño es único |
| `.unwrap()` en cualquier sitio · `.expect()` en producción | `?`, `match` o fallback explícito. `.expect("motivo")` solo si el fallo es **imposible por construcción** (p. ej. `write!` a un `String`), y el motivo lo dice |
| Tragarse un error (`let _ =`, `ok()`) · `panic!` para flujo esperado | `Result`; `Display`/`Error` a mano (el crate no tiene dependencias, no `thiserror`). `let _ =` solo en best-effort **documentado**: mensaje final cuando el workflow ya termina, eco de `--dry-run`, `JoinHandle` del keepalive |
| `pub` en un binario | `pub(crate)` |
| Mensajes que filtran secretos | Qué falló y con qué input, sin datos sensibles |
| Estado global mutado en tests (`set_var`, env, CWD) · fixtures sin mensaje | Configuración inyectada (`&dyn` ports, parámetros) y `.expect("contexto")` |
| Helpers de test que ignoran un `Result` | Que el test falle |
| Valores hardcodeados (URLs, timeouts, colores) | Constantes, configuración o `Palette`; los textos de UI viven en el step que los imprime |
| Nombre o versión a mano | `env!("CARGO_PKG_NAME")`, `env!("CARGO_PKG_VERSION")` |
| Dependencias injustificadas · inputs de flake que sobran | Cero deps salvo justificación fuerte; cada input justifica su mantenimiento |
| `cargo install`, `nix profile install`, `nix-env -i`, paquetes del sistema | Toda herramienta se declara en `flake.nix` |
| `--impure`, `builtins.getEnv`, rutas absolutas del host | El resultado depende solo del flake |
| Optimizar «por si acaso» | Benchmark o perfil primero |
| Lógica repetida ×2 | Extraer a función, tipo o módulo |
| Código muerto o comentado · `#[allow]` sin motivo | Borrar; `#[expect(clippy::lint, reason = "…")]` en el ítem |
| Warnings de `fmt`/`clippy`/`doc` | Cero: `-D warnings` |

- `String::from_utf8_lossy(&buf)`, `.is_ok()`/`.is_err()`/`.is_some()`, `.expect_err("contexto")`, `assert_ne!(x, "")` (clippy 1.99 rechaza `assert!(!x.is_empty())`).
- `impl Display`/`Error` manuales con `source()`: cero dependencias.
- Preferir `&mut dyn Write` a `<W: Write>`: las monomorfizaciones dejan regiones sin cubrir.

## 4. Entorno Nix

- `flake.nix` es el **único** sitio donde se declaran herramientas (toolchain de Rust, `clippy`, `rustfmt`, `cargo-llvm-cov`, LLVM).
- `devShells.default` lleva las herramientas de desarrollo (y exporta `LLVM_COV`/`LLVM_PROFDATA`); `packages.default`/`packages.rx` construyen el binario con `meta.mainProgram = "rx"`. Sin duplicar dependencias entre ambos.
- Salidas que deben seguir funcionando: `nix build`, `nix develop`, `nix flake check`, y los cross builds en `legacyPackages.<system>.cross.<target>`.
- `checks.<system>.default` refleja el paquete, así `nix flake check` construye algo real (no re-declara fmt/clippy/test: eso lo corre el CI).
- El paquete instala `rx.1` y las completions generadas por el propio binario, así no divergen del parser.
- Uso diario: `nix develop` (o `direnv` + `use flake`). Un comando suelto: `nix develop -c <cmd>`.
- `flake.lock` se commitea. Actualizar es una decisión explícita (`nix flake update`) revisando el diff de inputs.
- La versión de Rust la fija `rust-toolchain.toml` dentro del shell; ⛔ `rustup default` global.
- Si `llvm-profdata` del shell es más viejo que el LLVM de `rustc`, apunta `LLVM_COV`/`LLVM_PROFDATA` a un LLVM igual o más nuevo (el raw profile cambia de versión).

## 5. TDD y cobertura

Cada cambio empieza con un test en **rojo**, cada commit lleva sus tests, y nunca se avanza de feature con tests rojos.

```
1. RED      → el test falla por la razón correcta
2. GREEN    → mínimo código para que pase
3. REFACTOR → limpiar; los tests siguen verdes
```

```rust
#[test]
fn validate_rejects_empty_repo() {
    let config = Config { repo: String::new(), ..sample() };
    assert_eq!(
        config.validate().expect_err("empty repo").to_string(),
        "repo path cannot be empty"
    );
}
```

- **Cobertura 100 estricta** — regiones, funciones y líneas. Comando único:
  `cargo llvm-cov --all-targets --no-default-ignore-filename-regex`
  (sin la flag, cargo-llvm-cov ignora `tests.rs`/`tests/` y el código de test no cuenta).
- **Los tests cuentan**: no se excluyen. `#[coverage(off)]` no está disponible (toolchain pineado, sin nightly) y sería el último recurso.
- Preferir **arreglos estructurales** a excluir ramas: extraer funciones, mover el `?`. Una closure se instrumenta por sitio de expresión: comparte una `fn` nombrada para que sus ramas se fusionen entre tests.
- Cada `?` crea una rama: ambos caminos (Ok, Err) los ejercita algún test.
- Sin `#[should_panic]` (deja sin cubrir el retorno normal); si hace falta el camino de pánico, `std::panic::catch_unwind` en un test normal.
- Un test: `<verbo>_<sujeto>_<escenario>_<resultado>()`, un concepto y un `assert` cuando se pueda, determinista (sin red, sin sleeps, sin timestamps reales). Junto al módulo (`#[cfg(test)] mod tests` o `tests.rs` hermano) o en `tests/integration.rs` para el binario.
- Los helpers de test (p. ej. `ok_ui`) cubren sus dos ramas y fallan con mensaje.
- **Integración sin efectos**: nunca el workflow real. `--dry-run -y`, declinar por stdin (`n`), o un `--repo` inexistente. Pasa `--hostname` explícito cuando el test llegue al rebuild (el sandbox de Nix no tiene `hostname`).
- Sin tests de relleno ni overrides de `assert!`/`assert_eq!` para inflar cobertura. Lo no testeable se justifica: `// No test: <motivo>. Cubierto por <integración | manual | N/A>`.

| Testeable ✅ | No testeable ⛔ → aislar en un adaptador fino y cubrir con integración |
|---|---|
| Funciones puras, argv builders, parseo, `Display` | TTY real, prompt de sudo, `nixos-rebuild` de verdad |
| Puertos con fakes, `UserIo`, `CommandRunner` con stub | UI generada por macro, WASM y APIs de plataforma |

## 6. Inyección de dependencias

Todo servicio con efectos (I/O, procesos, tiempo, red) expone un trait; la capa de aplicación depende del trait, nunca de la implementación. `rx` es síncrono y de un solo hilo: **no** hay `async`, `Arc` ni `tokio`.

```rust
pub(crate) trait Nix {
    fn rebuild(&self, repo: &str, hostname: &str, options: BuildOptions) -> Result<(), Error>;
    fn password_pending(&self) -> Result<bool, Error>;
    fn refresh_credentials(&self) -> Result<(), Error>;
    // …
}

pub(crate) struct NixCli<'a> { runner: &'a dyn CommandRunner }

impl Nix for NixCli<'_> {
    fn rebuild(&self, repo: &str, hostname: &str, options: BuildOptions) -> Result<(), Error> {
        let status = self.runner.run(&cmd_rebuild(repo, hostname, options))?;
        ensure_success(status, Error::RebuildFailed)
    }
    // …
}
```

- El caso de uso recibe `&dyn Repo`, `&dyn Nix`, `&dyn UserIo`; `main` elige los adaptadores (proceso o dry-run).
- Los fakes viven en `src/system_update/application/tests.rs` (`MockRepo`, `MockNix`, `MockUi`) y se configuran **por constructor** (`Box<dyn Fn …>`), nunca mutando estado global ni env.
- Los fakes por defecto son funciones nombradas (`ok_str`, `no_password_pending`), para que sus ramas se fusionen entre tests.
- `CommandRunner` se decora (`ProcessRunner` → `DryRunRunner`) en la composición, no se convierte en puerto.

## 7. Calidad y cierre

Antes de cada commit, dentro del devShell (`nix develop`):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps --document-private-items   # sin warnings
cargo llvm-cov --all-targets --no-default-ignore-filename-regex
```

- `nix flake check` debe pasar (evalúa el paquete y el man page).
- CI (`.github/workflows/ci.yml`): `fmt`, `clippy -D warnings`, `test` y `coverage` con `--fail-under-lines/functions/regions 100`. Se queda en verde siempre.

**Terminado** cuando: hay un test que falla sin el cambio y pasa con él · cobertura 100 sin bajar · los cuatro comandos y `nix flake check` en verde · el código respeta las skills de Rust aplicables · no hay `unwrap`, `clone` ni dependencia nuevos sin justificar · README/rustdoc actualizados si cambió el comportamiento público · si cambió `version`, las notas del release las genera el workflow (no hay `CHANGELOG.md`).

## 8. Release y versionado

Un release se dispara por el **cambio de `version` en `Cargo.toml`**, nunca por un archivo aparte ni un tag manual.

- **CalVer `YY.MM.N`** (el esquema de memos, p. ej. `26.11.1`): año y mes del release más un contador para releases del mismo mes. No SemVer: `rx` no tiene API de librería cuyos breaks pueda anunciar. ⛔ Sufijo de letra (`26.11.0a`): semver lo lee como pre-release y ordena *antes*.
- `[package] version` es la **única fuente de verdad**: el binario la embebe con `env!("CARGO_PKG_VERSION")`, `flake.nix` la lee con `builtins.fromTOML`, y el workflow la parsea de la tabla `[package]`.
- Workflow (`.github/workflows/release.yml`):
  - Detecta la versión; si el tag `v<version>` ya existe → no-op en verde. **Idempotente**: re-ejecutar no duplica tags ni releases.
  - `workflow_dispatch` siempre construye la matriz; solo publica con `publish` y sin tag.
  - Matriz: `x86_64-unknown-linux-musl` (testeado y compilado nativo), `aarch64-unknown-linux-musl` (cross), `aarch64-apple-darwin` y `x86_64-apple-darwin` (runners macOS, sin firmar). Artefactos por target, `SHA256SUMS`, release en un job aparte.
  - Verifica antes de publicar (`cargo test --all-targets --locked`), construye, empaqueta `rx-v<version>-<target>.tar.gz` y publica.
  - Notas: **changelog categorizado**, no `--generate-notes`. `release-changelog-builder-action` en modo `COMMIT` agrupa los subjects convencionales entre el tag anterior y el release (breaking, feat, fix, perf, refactor, test, docs, style, build, ci, chore, revert). `git describe --tags --abbrev=0 HEAD^` fija el límite inferior; `toTag` es el SHA del commit del release porque el tag aún no existe, y el step de publish añade el link de diff tag-a-tag.
  - Cada `label_extractor` debe consumir el subject **entero** (`…:([\s\S]*)`): la acción *reemplaza* el match por el target, así que un patrón de solo prefijo produce un label basura y nada se categoriza. Un tipo de commit nuevo necesita su `categories`, o sus entradas van a uncategorized.
  - Sin cache de CI a propósito: cero dependencias y jobs de menos de un minuto; una cache añadiría una acción de terceros sin ganancia medible.
  - CI pinea rust 1.99 (`dtolnay/rust-toolchain@1.99`); mantenlo en sync con `rust-toolchain.toml`.
- ⛔ Bump desde el CI o commit de vuelta al repo · ⛔ tag a mano antes de que la versión esté en `Cargo.toml`.

**Commits**: `type(scope): description` (`feat`, `fix`, `test`, `refactor`, `docs`, `chore`, `perf`, `ci`, `build`, `style`, `revert`), en imperativo, un cambio coherente con sus tests, generado desde `git diff --cached`. **Preguntar siempre** antes de `git push`, antes de operaciones destructivas (purge, delete, force-push, rebase) y antes de tocar `name`/`version`. Nunca push sin permiso explícito.
