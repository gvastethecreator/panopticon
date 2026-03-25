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
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"
active_app_filter = "exe:c:\\program files\\arc\\arc.exe"
```

> `active_tag_filter` y `active_app_filter` son excluyentes. Si activas uno, el otro se limpia.

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

- crea un tag desde una app con el menú contextual;
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
