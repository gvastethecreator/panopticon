# Configuración

Panopticon guarda sus preferencias en archivos TOML locales. No existe una base de datos ni sincronización remota: la configuración persistida es parte del propio proyecto/usuario en Windows.

## Ubicaciones de configuración

### Perfil por defecto

```text
%APPDATA%\Panopticon\settings.toml
```

### Perfiles nombrados

```text
%APPDATA%\Panopticon\profiles\<perfil>.toml
```

### Fallback si `%APPDATA%` no existe

```text
%TEMP%\Panopticon\settings.toml
%TEMP%\Panopticon\profiles\<perfil>.toml
```

## Esquema general

```toml
initial_layout = "grid"
refresh_interval_ms = 2000
minimize_to_tray = true
close_to_tray = true
preserve_aspect_ratio = false
hide_on_select = true
animate_transitions = true
always_on_top = false
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"
active_app_filter = "exe:c:\\program files\\arc\\arc.exe"
group_windows_by = "application"
fixed_width = 320
fixed_height = 220
dock_edge = "left"
theme_id = "nord"
background_color_hex = "181513"
use_system_backdrop = true
show_toolbar = true
show_window_info = true
start_in_tray = false
background_image_path = "C:\\wallpapers\\workspace.png"
locked_layout = false
lock_cell_resize = false
show_app_icons = true

[tag_styles.work]
color_hex = "D29A5C"

[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
hide_on_select_override = false
tags = ["work", "browser"]
color_hex = "5CA9FF"
thumbnail_refresh_mode = "realtime"
thumbnail_refresh_interval_ms = 5000

[layout_customizations.Grid]
col_ratios = [0.7, 0.3]
row_ratios = [0.4, 0.6]
```

## Claves globales

| Clave | Tipo | Default | Efecto real en runtime | Notas |
| --- | --- | --- | --- | --- |
| `initial_layout` | `LayoutType` | `Grid` | layout activo al arrancar | también se actualiza al cambiar layout en runtime |
| `refresh_interval_ms` | `u32` | `2000` | frecuencia de `refresh_windows()` | se normaliza a `1000`, `2000`, `5000` o `10000` |
| `minimize_to_tray` | `bool` | `true` | minimizar esconde la app en tray | afecta `WM_SIZE` |
| `close_to_tray` | `bool` | `true` | cerrar esconde la app en tray | afecta `WM_CLOSE` |
| `preserve_aspect_ratio` | `bool` | `false` | default global para miniaturas | se puede sobrescribir por app |
| `hide_on_select` | `bool` | `true` | oculta Panopticon al activar una app | en modo dock queda desactivado de forma efectiva |
| `animate_transitions` | `bool` | `true` | anima cambios de layout | duración actual: `180 ms` |
| `always_on_top` | `bool` | `false` | usa `SetWindowPos(HWND_TOPMOST)` | también afecta diálogos secundarios |
| `active_monitor_filter` | `Option<String>` | `None` | filtra ventanas por monitor | persistente entre sesiones |
| `active_tag_filter` | `Option<String>` | `None` | filtra ventanas por tag | excluyente con `active_app_filter` |
| `active_app_filter` | `Option<String>` | `None` | filtra ventanas por app | excluyente con `active_tag_filter` |
| `group_windows_by` | `WindowGrouping` | `None` | reordena ventanas visibles | no filtra; solo agrupa/ordena |
| `fixed_width` | `Option<u32>` | `None` | grosor del dock lateral | hoy no fija el ancho de la ventana flotante |
| `fixed_height` | `Option<u32>` | `None` | grosor del dock superior/inferior | hoy no fija la altura de la ventana flotante |
| `dock_edge` | `Option<DockEdge>` | `None` | activa modo appbar | valores: `left`, `right`, `top`, `bottom` |
| `theme_id` | `Option<String>` | `None` | selecciona preset de `assets/themes.json` | `None` = tema clásico |
| `background_color_hex` | `String` | `181513` | color base del cliente | también participa en fallback del tema clásico |
| `use_system_backdrop` | `bool` | `true` | backdrop + rounded corners en Windows 11 | vía `DwmSetWindowAttribute` |
| `show_toolbar` | `bool` | `true` | muestra/oculta header superior | también altera espacio útil del viewport |
| `show_window_info` | `bool` | `true` | muestra título/app sobre la miniatura | afecta el alto útil del thumbnail |
| `start_in_tray` | `bool` | `false` | arranca escondido | libera thumbnails antes de ocultar |
| `background_image_path` | `Option<String>` | `None` | dibuja imagen detrás del tablero | si falla la carga se limpia silenciosamente |
| `locked_layout` | `bool` | `false` | bloquea cambios de layout | desactiva atajos y toolbar para layouts |
| `lock_cell_resize` | `bool` | `false` | bloquea drag de separadores | puede coexistir con `locked_layout` |
| `show_app_icons` | `bool` | `true` | muestra iconos en tarjetas | usa caché + rasterización GDI |
| `layout_customizations` | `Map<String, LayoutCustomization>` | vacío | ratios personalizados por layout | se generan al arrastrar separadores |
| `app_rules` | `Map<String, AppRule>` | vacío | reglas persistentes por app | clave = `app_id` |
| `tag_styles` | `Map<String, TagStyle>` | vacío | color de tags manuales | se normaliza según tags realmente existentes |

## Reglas por aplicación

Cada entrada de `app_rules` está indexada por `app_id`. El formato más común es:

```toml
[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
hide_on_select_override = false
tags = ["work", "browser"]
color_hex = "5CA9FF"
thumbnail_refresh_mode = "interval"
thumbnail_refresh_interval_ms = 5000
```

### Campos de `AppRule`

| Campo | Tipo | Uso |
| --- | --- | --- |
| `display_name` | `String` | etiqueta amigable en menús y restore |
| `hidden` | `bool` | excluye la app del layout visible |
| `preserve_aspect_ratio` | `bool` | sobrescribe el default global |
| `hide_on_select` | `bool` | valor heredado/compatibilidad |
| `hide_on_select_override` | `Option<bool>` | override explícito moderno |
| `tags` | `Vec<String>` | tags manuales para agrupación/filtro |
| `color_hex` | `Option<String>` | color fijo para la tarjeta |
| `thumbnail_refresh_mode` | `ThumbnailRefreshMode` | `realtime`, `frozen`, `interval` |
| `thumbnail_refresh_interval_ms` | `Option<u32>` | usado cuando el modo es `interval` |

## Tags y estilos

Los tags manuales se almacenan en `tag_styles` y se asignan a apps desde `app_rules.<app>.tags`.

```toml
[tag_styles.work]
color_hex = "D29A5C"

[tag_styles.browser]
color_hex = "5CA9FF"
```

### Comportamiento importante

- los tags se normalizan a minúsculas;
- se eliminan duplicados y espacios sobrantes;
- si un tag deja de usarse en `app_rules`, su estilo puede desaparecer tras normalización;
- cuando `active_tag_filter` está activo, el color del tag se usa como accent por defecto del tablero filtrado.

## Layout customizations

Los layouts con separadores persistibles escriben sus ratios bajo `layout_customizations`.

```toml
[layout_customizations.Columns]
col_ratios = [0.2, 0.5, 0.3]

[layout_customizations.Row]
col_ratios = [0.15, 0.35, 0.2, 0.3]
```

Notas:

- `Grid` usa `col_ratios` y `row_ratios`.
- `Mosaic` persiste principalmente `row_ratios`.
- `Bento` usa una razón principal de columna y ratios para el sidebar.
- `Columns` usa ratios de columnas.
- `Row` y `Column` persisten ratios por item en el eje scrollable.
- `Fibonacci` no usa customización persistente de separadores.

## Perfiles

Panopticon soporta perfiles separados por archivo.

### Cómo se usan

- la app puede arrancar con `--profile <nombre>`;
- la ventana de settings permite guardar el estado actual en otro perfil;
- también permite abrir otra instancia con otro perfil;
- si no hay perfiles extra, el runtime intenta sembrar `profile-1` y `profile-2`.

## Normalización automática

Antes de entrar al runtime, `AppSettings::normalized()` corrige varios casos:

- intervalos de refresco fuera del conjunto permitido;
- filtros vacíos;
- tags vacíos o duplicados;
- colores hex inválidos;
- nombres de perfil no válidos para Windows;
- reglas con `app_id` vacío;
- estilos de tags huérfanos;
- herencia antigua de `hide_on_select` hacia el modelo moderno con override.

## Relaciones y exclusiones importantes

### Filtros

- activar `active_tag_filter` limpia `active_app_filter`;
- activar `active_app_filter` limpia `active_tag_filter`;
- `active_monitor_filter` puede coexistir con los anteriores.

### Dock

- si `dock_edge` está activo, `hide_on_select_for(app)` devuelve `false` en runtime aunque exista override;
- `fixed_width` / `fixed_height` se interpretan como grosor del dock, no como tamaño libre de la ventana flotante.

### Temas

- `theme_id = None` usa el tema clásico;
- si `theme_id` no existe en `assets/themes.json`, el runtime hace fallback al tema clásico.
