# Configuración

Panopticon guarda sus preferencias en:

```text
%APPDATA%\Panopticon\settings.toml
```

## Campos globales

```toml
initial_layout = "grid"
refresh_interval_ms = 2000
minimize_to_tray = true
close_to_tray = true
preserve_aspect_ratio = false
hide_on_select = true
animate_transitions = true
always_on_top = false
background_color_hex = "181513"
use_system_backdrop = true
show_toolbar = true
show_window_info = true
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"
active_app_filter = "exe:c:\\program files\\arc\\arc.exe"

[tag_styles.work]
color_hex = "D29A5C"
```

> `active_tag_filter` y `active_app_filter` son excluyentes. Si activas uno, el otro se limpia.

## Apariencia y UX

- `background_color_hex` define el fondo base del cliente en formato `RRGGBB`.
- `use_system_backdrop` intenta aplicar fondo estilo Windows 11 y esquinas redondeadas.
- `show_toolbar` muestra u oculta el header superior.
- `show_window_info` muestra u oculta el título / app debajo de cada thumbnail.
- Cuando `active_tag_filter` está activo, el área de contenido se tiñe con el color del tag seleccionado.

## Reglas por aplicación

```toml
[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
tags = ["work", "browser"]
```

## Estrategia de grupos

### Manual con tags

- crea un tag desde una app con el diálogo **Create custom tag…**;
- elige nombre + color al crearlo;
- asígnalo a otras apps desde **Assign existing tags**;
- filtra el tablero desde el tray con **Filter by tag**.

### Automática por aplicación

Usa **Filter by application** para ver únicamente las ventanas de una misma app, usando el `app_id` persistente del sistema.

## Normalización automática

Panopticon limpia automáticamente:

- filtros vacíos,
- tags vacíos,
- duplicados de tags,
- espacios sobrantes.
