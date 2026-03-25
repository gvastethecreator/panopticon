# Getting Started

## Requisitos

- Windows 10/11
- Rust estable

## Primer arranque

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release
```

## Qué esperar

Al iniciar, Panopticon:

1. enumera ventanas visibles,
2. crea miniaturas DWM,
3. calcula un layout inicial,
4. muestra el tablero y registra el icono del tray.

## Primeras acciones recomendadas

- prueba cambiar de layout con `Tab`;
- oculta una app con click derecho;
- crea un tag desde una app;
- filtra por monitor o por app desde el tray;
- revisa el archivo `settings.toml` para entender la persistencia.
