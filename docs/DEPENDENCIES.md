# Dependency maintenance

Revisión: 2026-08-14. Panopticon es un binario Rust/Slint para Windows; Cargo es el gestor de la
aplicación. No hay Bun en el proyecto. La configuración privada `.opencode/` no participa del binario:
usa pnpm 11.20.0, `pnpm-lock.yaml` y `@opencode-ai/plugin` 1.18.16.

## Directas

| Crate | Versión | Uso | Resultado |
| --- | ---: | --- | --- |
| `slint` / `slint-build` | 1.17.1 | UI y compilación de `.slint` | latest compatible |
| `raw-window-handle` | 0.6.2 | puente de HWND | latest compatible |
| `thiserror` | 2.0.20 | errores de dominio | actualizado desde 2.0.19 |
| `anyhow` | 1.0.104 | errores de arranque | latest compatible |
| `serde` | 1.0.229 | serialización TOML/JSON | latest compatible |
| `serde_json` | 1.0.151 | datos auxiliares | latest compatible |
| `toml` | 1.1.4 | settings/workspaces | latest compatible |
| `tracing` | 0.1.44 | diagnóstico | latest compatible |
| `tracing-subscriber` | 0.3.23 | filtros/formato de logs | latest compatible |
| `tracing-appender` | 0.2.5 | archivos de log | latest compatible |
| `rfd` | 0.17.2 | selector de archivos nativo | latest compatible |
| `windows` | 0.62.2 | Win32/DWM/Shell | latest compatible |
| `winres` | 0.1.12 | recursos PE Windows | latest compatible |

La actualización inicial del 11 de agosto refrescó 107 paquetes compatibles. Una revisión previa del
14 de agosto añadió 25 actualizaciones; este lote agregó cuatro updates transitivos adicionales:
`cc` 1.4.3, `find-msvc-tools` 0.1.11, `libredox` 0.1.20 y `pkg-config` 0.3.34. La API persistida no
cambió y los 170 tests compilan con el lock resultante.

Slint queda con `default-features = false` y activa de forma explícita `accessibility`, `compat-1-2`,
`raw-window-handle-06`, `backend-winit` y `renderer-skia`. Esto retira Femtovg, software renderer,
backend default y system tray de Slint; Panopticon conserva Skia y su integración tray Win32 propia.

## Changelogs revisados

- [Slint 1.17.1 changelog](https://github.com/slint-ui/slint/blob/master/CHANGELOG.md): correcciones
  de crash al iniciar, bindings, `TextEdit`, layout, popups y sincronización de timers; se mantienen
  las features Winit/Skia actuales.
- [Rust for Windows releases](https://github.com/microsoft/windows-rs/releases): la línea 0.62
  actualiza `windows-core`/metadata y el soporte de linking; no requiere cambios en los wrappers
  Win32 existentes.
- [thiserror releases](https://github.com/dtolnay/thiserror/releases) y [anyhow releases](https://github.com/dtolnay/anyhow/releases):
  mantenimiento de macros/tipos de error, sin cambios de contrato en nuestros `Result`.
- [Serde releases](https://github.com/serde-rs/serde/releases) y [serde_json releases](https://github.com/serde-rs/json/releases):
  mejoras de compatibilidad y requisitos mínimos; las estructuras persistidas se cubren con roundtrip.
- [toml releases](https://github.com/toml-rs/toml/releases), [tracing releases](https://github.com/tokio-rs/tracing/releases),
  [rfd releases](https://github.com/PolyMeilex/rfd/releases) y [raw-window-handle releases](https://github.com/rust-windowing/raw-window-handle/releases):
  mantenimiento de parser, logging, diálogos y handles; no hay migración de API necesaria.

## Contrato de actualización

```powershell
cargo update -n
cargo update
cargo fmt -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic
cargo test --all-targets --locked
cargo build --release --locked
cargo doc --no-deps --locked
cargo audit
```

`cargo audit` pasó localmente con 0 vulnerabilidades y cuatro avisos `unmaintained` permitidos
(`bincode`, `paste`, `rustybuzz`, `ttf-parser`) de la cadena transitoria de Slint. CI lo repite en
Windows. Las actualizaciones deben conservar `rust-toolchain.toml` (Rust 1.96.0) y validar el runtime
nativo en Windows.
