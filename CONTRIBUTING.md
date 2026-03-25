# Contribuir a Panopticon

Gracias por dedicarle tiempo a Panopticon.

## Antes de empezar

- Lee el `README.md`.
- Revisa `docs/ARCHITECTURE.md` si vas a tocar Win32, DWM o persistencia.
- Si el cambio afecta settings, actualiza también `docs/CONFIGURATION.md`.

## Setup local

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo check
cargo test
```

## Reglas del proyecto

### Calidad mínima

Antes de abrir PR, ejecuta:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings -W clippy::pedantic
cargo test --all-targets
```

### `unsafe`

Todo bloque `unsafe` debe explicar su invariante con `// SAFETY:`.

### Cambios visibles

Si cambias una feature visible:

- actualiza `README.md`,
- documenta el comportamiento nuevo o cambiado,
- añade tests si el cambio toca lógica pura o persistencia.

### Estilo de PR

- mantén los cambios pequeños y coherentes;
- explica el **por qué**, no sólo el **qué**;
- evita mezclar refactors no relacionados con una feature o fix.

## Zonas importantes del código

- `src/main.rs`: window loop, input, repaint, runtime state.
- `src/window_enum.rs`: descubrimiento y filtrado de ventanas.
- `src/settings.rs`: persistencia TOML, reglas por app, tags y filtros.
- `src/app/tray.rs`: integración con tray y menús.
- `src/layout.rs`: layouts puros y testeables.

## Ideas de contribución útiles

- mejorar el editor de tags dentro de la UI,
- empaquetado e instalador,
- telemetría totalmente opt-in para diagnósticos,
- capturas o GIFs para documentación,
- más tests para settings y flujos de filtros.

## Reporte de bugs

Usa la plantilla de issue correspondiente e incluye:

- Windows version,
- pasos exactos para reproducir,
- logs relevantes de `%TEMP%/panopticon/logs/`.
