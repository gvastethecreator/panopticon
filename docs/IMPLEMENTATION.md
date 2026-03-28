# Implementación

Este documento describe cómo está implementado Panopticon hoy: qué hace cada módulo, cómo se mueve el estado por el runtime y qué decisiones prácticas sostienen la funcionalidad visible.

## Panorama general

El binario principal está concentrado en `src/main.rs`, que orquesta:

- creación y vida de la ventana principal;
- timers periódicos;
- sincronización entre estado, UI Slint y miniaturas DWM;
- integración con tray, dock/appbar, menús nativos y diálogos secundarios;
- persistencia de settings y perfiles.

La implementación se apoya en varios módulos especializados para evitar que toda la lógica viva pegada a Win32.

## Bootstrap del binario

El arranque de `main()` sigue este orden conceptual:

1. inicializar logging;
2. leer `--profile` si existe;
3. activar DPI awareness y registrar `TaskbarCreated`;
4. crear iconos de aplicación;
5. cargar settings desde TOML;
6. resolver el tema activo;
7. crear `MainWindow` de Slint;
8. sincronizar propiedades iniciales desde `AppSettings`;
9. crear `AppState` compartido (`Rc<RefCell<_>>`);
10. mostrar la ventana y registrar callbacks;
11. posponer la inicialización que depende del `HWND` real;
12. arrancar los timers recurrentes.

Ese patrón de inicialización diferida es importante: Slint necesita crear primero la ventana para que luego Rust pueda extraer su `HWND` y enchufarle las piezas Win32.

## Estado principal

### `ManagedWindow`

Cada ventana visible en el tablero se modela como una estructura enriquecida que combina:

- metadatos persistibles (`WindowInfo`);
- thumbnail DWM opcional;
- rectángulos de target/display para animación;
- tamaño fuente del thumbnail;
- timestamps y cachés para refresco;
- icono rasterizado para fallback y cabeceras.

### `AppState`

`AppState` es el agregado central del runtime. Contiene:

- `hwnd` principal;
- `windows: Vec<ManagedWindow>`;
- layout actual;
- hover y ventana activa;
- tray icon;
- settings cargados;
- estado de animación;
- estado de scroll y contenido;
- separadores y drag state;
- background image cargada;
- tema actual y transición de tema.

En términos prácticos, `AppState` es la fuente de verdad operativa del programa.

## Enumeración y reconciliación de ventanas

`refresh_windows()` es uno de los centros de la implementación. Su trabajo es:

1. llamar a `enumerate_windows()`;
2. descartar la propia ventana de Panopticon;
3. refrescar nombres amigables de apps conocidos en `settings`;
4. aplicar filtros persistentes por monitor, tag, app y `hidden`;
5. ordenar el resultado si `group_windows_by` está activo;
6. reconciliar el vector actual de `ManagedWindow` con el descubrimiento nuevo;
7. crear o mantener thumbnails cuando corresponde.

### `window_enum.rs`

Este módulo implementa el callback de `EnumWindows` y obtiene:

- título;
- clase;
- proceso/ruta ejecutable;
- `app_id` estable;
- monitor.

El `app_id` se construye con una prioridad concreta:

1. `exe:<ruta ejecutable en minúsculas>`
2. `class:<clase>`
3. `title:<título>`

Eso hace que las reglas por aplicación sean relativamente estables entre sesiones.

## Miniaturas DWM

### Wrapper RAII

`src/thumbnail.rs` encapsula el handle `HTHUMBNAIL` y garantiza `DwmUnregisterThumbnail` en `Drop`.

### Registro y actualización

`main.rs` decide cuándo crear o liberar un `Thumbnail`:

- se crea cuando una ventana gestionada necesita visualizarse y aún no tiene thumbnail;
- se libera si la app se oculta, si la ventana fuente se minimiza o si la actualización falla.

### Modos de refresco por app

La implementación soporta tres modos en settings:

- `Realtime`
- `Frozen`
- `Interval`

Aunque la UI actual no expone un editor visual directo para estos modos, el runtime sí los respeta si están presentes en el TOML.

### Cálculo de rectángulos DWM

`compute_dwm_rect()` traduce la tarjeta lógica Slint al rectángulo físico de DWM teniendo en cuenta:

- padding interno;
- franja de accent superior;
- banda informativa si hay título/icono;
- viewport por scroll;
- altura de toolbar;
- factor de escala DPI;
- preserve-aspect ratio.

## Layout engine

`src/layout.rs` es el motor geométrico del proyecto.

### Qué devuelve

`compute_layout_custom()` produce:

- `rects`: rectángulos destino por ventana;
- `separators`: handles lógicos para permitir resize persistente.

### Layouts disponibles

- `Grid`
- `Mosaic`
- `Bento`
- `Fibonacci`
- `Columns`
- `Row`
- `Column`

### Persistencia de ratios

Cuando el usuario arrastra un separador:

1. `main.rs` garantiza que existan ratios base para ese layout;
2. actualiza `layout_customizations`;
3. normaliza settings;
4. persiste el resultado en disco;
5. recompone la UI inmediatamente.

Para layouts de tira (`Row`, `Column`) y columnas, el proyecto usa un ajuste grupal (`apply_separator_drag_grouped`) para que el cambio se redistribuya con un comportamiento más natural.

## UI Slint

`ui/main.slint` define tres ventanas principales:

- `MainWindow`
- `SettingsWindow`
- `TagDialogWindow`

También define los componentes reutilizables que estructuran la UX:

- `ThumbnailCard`
- `ResizeHandle`
- `Toolbar`
- `OverlayScrollbar`
- `ContextMenuOverlay`
- `EmptyState`

### Nota importante sobre menús

La interacción principal de menús se resuelve mediante menús nativos Win32 implementados en `src/app/tray.rs` y `src/app/window_menu.rs`. La UI declarativa mantiene el tablero y los diálogos, pero el flujo real de menús ya no depende de overlays Slint.

## Interacción por teclado y mouse

La implementación soporta atajos directos en `handle_key()`:

- selección de layouts;
- reset de customizaciones;
- toggles de animación, toolbar, info y topmost;
- apertura de settings y menú principal;
- refresco manual;
- cambio de tema;
- salida.

Además, la subclase Win32 intercepta rueda y botón central para permitir pan/scroll en layouts con overflow.

## Settings, perfiles y diálogos

### `settings.rs`

Este módulo define:

- `AppSettings`
- `AppRule`
- `TagStyle`
- `ThumbnailRefreshMode`
- `WindowGrouping`
- `DockEdge`

También resuelve:

- path por defecto y por perfil;
- listado de perfiles;
- guardado/carga TOML;
- normalización de datos;
- helpers de negocio (`toggle_hidden`, `toggle_app_tag`, `set_tag_filter`, etc.).

### `settings_ui.rs`

Es la capa adaptadora entre `AppSettings` y `SettingsWindow`. Traduce enums a índices de combo box y viceversa.

### Settings window

La ventana de configuración permite editar:

- comportamiento general;
- display;
- tema;
- layout por defecto;
- refresh interval;
- fixed dimensions;
- dock edge;
- filtros;
- background;
- perfiles;
- hidden apps.

### Perfil múltiple

La app puede:

- guardar el estado actual en un perfil;
- lanzar otra instancia con `--profile`;
- mostrar una etiqueta del perfil actual.

## Tray y menús nativos

### `src/app/tray.rs`

Gestiona:

- creación del icono de tray con `Shell_NotifyIconW`;
- re-registro tras reinicio de Explorer;
- menú contextual del tray;
- creación de iconos generados en memoria;
- extracción de iconos desde ventanas o ejecutables.

### `src/app/window_menu.rs`

Define el menú por ventana, con acciones como:

- ocultar del layout;
- alternar preserve-aspect;
- alternar hide-on-select;
- crear tag personalizada;
- cambiar color de la tarjeta;
- cerrar ventana;
- matar proceso.

## Dock / appbar

El modo dock está implementado dentro de `main.rs` usando `SHAppBarMessage`.

Cuando se activa:

- la ventana cambia a estilo `WS_POPUP | WS_VISIBLE`;
- se registra como appbar;
- reserva espacio del escritorio;
- se reposiciona ante cambios del entorno;
- ciertos comandos del menú del sistema quedan bloqueados;
- `hide_on_select` se invalida de forma efectiva.

### Matiz importante

`fixed_width` y `fixed_height` hoy actúan como grosor del dock, no como tamaño forzado de la ventana flotante fuera del modo appbar.

## Theming e iconografía

### `theme.rs`

Construye un `UiTheme` a partir de:

- un preset embebido en `assets/themes.json`, o
- el tema clásico derivado de `background_color_hex`.

También implementa interpolación entre temas para hacer transiciones suaves.

### Render de iconos

La app puede:

- pedir el icono a la propia ventana;
- pedirlo a la clase Win32;
- extraerlo del ejecutable;
- rasterizarlo a RGBA para Slint usando GDI.

Esto permite mostrar iconos sobre tarjetas y placeholders de ventanas minimizadas.

## Logging

`logging.rs` configura `tracing` con appender rolling diario. El logger queda vivo durante toda la vida del proceso gracias a `WorkerGuard`.

## Calidad y pruebas

### Cobertura existente

- `tests/layout_tests.rs` cubre el motor de layouts;
- `src/settings.rs` prueba persistencia TOML, normalización y reglas;
- `src/theme.rs` valida catálogo e interpolación.

### Gaps actuales

No hay una cobertura comparable para:

- tray icon y menús nativos;
- enumeración Win32 real;
- thumbnails DWM;
- appbar/dock;
- icon extraction y render GDI.

## Qué convendría refactorizar a futuro

1. dividir `main.rs` por responsabilidades;
2. extraer una capa de estado/acciones más explícita;
3. desacoplar aún más la lógica de menú y diálogos;
4. aclarar o eliminar piezas declarativas no conectadas desde la UI Slint;
5. cerrar el hueco entre settings persistidos y opciones realmente expuestas en la UI.
