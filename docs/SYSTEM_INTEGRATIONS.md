# Integraciones del sistema y dependencias

Panopticon no depende de servicios externos. Su integración principal es con el propio sistema operativo Windows y con un conjunto pequeño de crates del ecosistema Rust.

## Resumen ejecutivo

- **Backend remoto:** no existe
- **Base de datos:** no existe
- **API HTTP:** no existe
- **Servicios del sistema:** sí, varios subsistemas de Windows
- **Persistencia local:** archivos TOML y logs en disco

## APIs y subsistemas de Windows utilizados

### Desktop Window Manager (DWM)

| API | Uso en Panopticon |
| --- | --- |
| `DwmRegisterThumbnail` | registrar miniaturas vivas entre ventanas |
| `DwmUpdateThumbnailProperties` | mover, mostrar/ocultar y adaptar cada thumbnail |
| `DwmUnregisterThumbnail` | liberar recursos al destruir o soltar thumbnails |
| `DwmQueryThumbnailSourceSize` | consultar tamaño de la ventana origen |
| `DwmSetWindowAttribute` | aplicar dark mode, backdrop y esquinas redondeadas |

**Rol funcional:** núcleo del producto. Sin DWM, Panopticon pierde su propuesta principal de miniaturas vivas.

### User32 / Windows and Messaging

| API | Uso |
| --- | --- |
| `EnumWindows` | descubrir ventanas de nivel superior |
| `GetWindowTextW` / `GetClassNameW` | obtener metadatos de ventanas |
| `GetWindowLongW` / `GetWindowLongPtrW` / `SetWindowLongPtrW` | leer estilos y subclasificar la ventana principal |
| `SetForegroundWindow` / `ShowWindow` | activar y restaurar ventanas |
| `PostMessageW(WM_CLOSE)` | pedir cierre elegante de una ventana |
| `SetWindowPos` | topmost, reposicionamiento, docking y centrado |
| `TrackPopupMenu` / `AppendMenuW` | construir menús nativos |
| `RegisterWindowMessageW("TaskbarCreated")` | detectar reinicio de Explorer |

**Rol funcional:** control de ventanas, eventos y comportamiento nativo.

### Shell / AppBar / System Tray

| API | Uso |
| --- | --- |
| `Shell_NotifyIconW` | registrar/eliminar icono de tray |
| `SHAppBarMessage` | modo dock/appbar |
| `ExtractIconExW` | extraer iconos desde ejecutables |

**Rol funcional:** convertir Panopticon en una utilidad de escritorio persistente, no solo en una ventana más.

### GDI

| API | Uso |
| --- | --- |
| `GetDC`, `CreateCompatibleDC`, `CreateDIBSection` | crear superficie temporal para rasterizar iconos |
| `DrawIconEx` | pintar iconos a un buffer intermedio |
| `SelectObject`, `DeleteDC` | gestión de recursos GDI |
| `GetMonitorInfoW`, `MonitorFromWindow` | geometría de monitor y soporte para dock/DPI |

**Rol funcional:** soporte visual complementario, sobre todo para iconos y geometría de pantalla.

### HiDPI

| API | Uso |
| --- | --- |
| `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` | cálculo correcto en monitores con escalas distintas |

### Threading / Process introspection

| API | Uso |
| --- | --- |
| `OpenProcess` | consultar ruta ejecutable o terminar proceso |
| `QueryFullProcessImageNameW` | obtener ruta del binario fuente |
| `TerminateProcess` | acción de menú “Kill process” |

## Dependencias Rust relevantes

| Crate | Papel |
| --- | --- |
| `slint` | framework UI declarativo principal |
| `slint-build` | compilación de `ui/main.slint` en build time |
| `windows` | bindings oficiales para Win32/DWM/Shell/GDI |
| `raw-window-handle` | acceso al `HWND` generado por Slint |
| `serde` | serialización/deserialización de settings |
| `serde_json` | carga del catálogo de temas |
| `toml` | persistencia del archivo de configuración |
| `tracing` | logging estructurado |
| `tracing-subscriber` | subscriber y filtros de logging |
| `tracing-appender` | escritura rolling a archivo |
| `thiserror` | errores tipados del crate |
| `anyhow` | errores cómodos a nivel aplicación |

## Integración con el sistema de archivos

### Settings

Panopticon guarda settings en:

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\profiles\<perfil>.toml
```

### Logs

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

### Assets embebidos o leídos localmente

| Ruta | Uso |
| --- | --- |
| `assets/themes.json` | catálogo de temas base |
| `ui/main.slint` | UI compilada en build time |
| `background_image_path` | imagen opcional cargada desde disco por el usuario |

## Integración de build

`build.rs` ejecuta:

```rust
slint_build::compile("ui/main.slint")
```

Esto significa que cualquier error de sintaxis o bindings en Slint se vuelve parte del pipeline de compilación normal de Cargo.

## Seguridad y `unsafe`

Panopticon usa `unsafe` por necesidad, no como estilo general. Las integraciones que lo requieren son:

- callbacks FFI (`EnumWindows`, subclase Win32);
- DWM thumbnail management;
- manipulación de handles, iconos y menús nativos;
- operaciones GDI;
- atributos DWM de ventanas.

### Criterio general observable en el código

- bloques `unsafe` relativamente pequeños;
- comentarios `SAFETY` en zonas críticas;
- wrappers seguros cuando tiene sentido, como `Thumbnail`.

## Implicaciones operativas

### Lo que el sistema debe permitir

- DWM activo;
- entorno gráfico de Windows real;
- acceso normal a ventanas de usuario.

### Limitaciones del sistema

- UIPI puede limitar interacciones con ventanas elevadas;
- ciertos comportamientos dependen del compositor de Windows y del estado de la ventana origen;
- el tray depende de Explorer; por eso existe lógica de re-registro.

## Qué no utiliza el proyecto

Conviene dejarlo explícito porque ayuda a entender su naturaleza:

- no hay HTTP client;
- no hay servicios cloud;
- no hay IPC de red;
- no hay base de datos SQL/NoSQL;
- no hay telemetría remota;
- no hay autentificación ni cuentas de usuario.

Panopticon es, en esencia, una utilidad local rica en integración con Windows y bastante austera en dependencias fuera de ese mundo.
