# Arquitectura de Panopticon

## Resumen

Panopticon es una aplicación Win32 nativa escrita en Rust. Combina:

- enumeración de ventanas (`EnumWindows`),
- miniaturas DWM (`DwmRegisterThumbnail`),
- layout puro en memoria,
- render UI ligero con GDI,
- persistencia TOML para reglas y filtros.

## Estructura del crate

```text
src/
├── app/
│   ├── mod.rs
│   └── tray.rs
├── constants.rs
├── error.rs
├── layout.rs
├── lib.rs
├── logging.rs
├── main.rs
├── settings.rs
├── thumbnail.rs
└── window_enum.rs
```

## Flujo principal

```text
EnumWindows
  → WindowInfo[]
  → filtros (monitor / tag / app / hidden)
  → ManagedWindow[]
  → compute_layout(...)
  → DwmUpdateThumbnailProperties(...)
  → WM_PAINT para labels, bordes y toolbar
```

## Responsabilidades por módulo

### `src/window_enum.rs`

- Descubre ventanas visibles de usuario.
- Excluye superficies del sistema y tool windows.
- Deriva un `app_id` estable.
- Obtiene el `monitor_name` para filtros por monitor.

### `src/settings.rs`

- Persiste preferencias globales y por aplicación.
- Normaliza settings de disco.
- Gestiona reglas por app:
  - `hidden`
  - `preserve_aspect_ratio`
  - `hide_on_select`
  - `tags`
- Guarda filtros activos:
  - `active_monitor_filter`
  - `active_tag_filter`
  - `active_app_filter`

### `src/main.rs`

- Crea la ventana principal Win32.
- Mantiene `AppState` asociado al `HWND` con `GWLP_USERDATA`.
- Atiende el message loop.
- Sincroniza ventanas, thumbnails, layout y repaint.
- Implementa el menú contextual por aplicación.

### `src/app/tray.rs`

- Registra el icono del tray.
- Reacciona a `TaskbarCreated` para re-registrar el icono tras reinicio de Explorer.
- Construye menús para:
  - acciones rápidas,
  - restauración de apps ocultas,
  - filtro por monitor,
  - filtro por tag,
  - filtro por aplicación.

### `src/layout.rs`

Motor puro y testeable:

```text
(LayoutType, RECT, count, aspects) -> Vec<RECT>
```

No depende de Win32 más allá del tipo geométrico `RECT`, por eso es la parte más sencilla de validar en tests.

## Decisiones de diseño importantes

### Estado acoplado al `HWND`

Se evita un singleton global. El estado vive en `GWLP_USERDATA`, lo que sigue el patrón clásico de Win32 y hace más predecible la liberación en `WM_NCDESTROY`.

### Miniaturas DWM en vez de capturas bitmap

La composición queda en GPU y evita costes extra de CPU o copia de buffers.

### Filtros como parte de la configuración persistida

Los filtros no sólo afectan la vista actual: también sobreviven entre sesiones. Esto permite que Panopticon arranque “ya filtrado” si así lo dejó la persona usuaria.

### Tags manuales + grupos automáticos

- **Manual**: cada app puede tener `tags` persistentes.
- **Automático**: el filtro por aplicación reutiliza `app_id` y `display_name` ya existentes.

Eso reduce complejidad de UI y evita introducir una estructura de grupos duplicada.

## Seguridad y `unsafe`

El proyecto usa `unsafe` por necesidad, no por deporte. Las reglas activas son:

- cada bloque `unsafe` debe llevar comentario `// SAFETY:`;
- no se exponen raw pointers en APIs públicas;
- la lógica de alto nivel sigue estando fuera de `unsafe` siempre que es posible.

## Oportunidades futuras

- editor de tags totalmente libre dentro de la UI (sin depender del TOML ni de tags sembrados desde apps),
- perfiles por monitor,
- empaquetado/instalador,
- screenshots y demos automatizadas,
- publicación de releases firmadas.
