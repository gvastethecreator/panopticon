# Primeros pasos

Esta guía sirve para compilar, ejecutar y entender el flujo inicial de Panopticon sin tener que recorrer primero todo el código fuente.

## Requisitos

- Windows 10 o Windows 11 (64-bit)
- DWM habilitado
- toolchain Rust estable
- un escritorio real con ventanas de usuario abiertas para que Panopticon tenga algo que mostrar

## Clonado y ejecución

### Modo desarrollo

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run
```

### Modo release

```bash
cargo run --release
```

### Ejecutar con un perfil concreto

```bash
cargo run --release -- --profile trabajo
```

Esto hace que la configuración se cargue desde `%APPDATA%\Panopticon\profiles\trabajo.toml`.

## Qué ocurre en el arranque

En una sesión normal, Panopticon hace lo siguiente:

1. inicializa logging en `%TEMP%\panopticon\logs\`;
2. activa DPI awareness por monitor;
3. carga settings y perfil activo desde TOML;
4. crea la ventana principal Slint;
5. adquiere el `HWND` nativo y aplica apariencia DWM;
6. registra el icono de tray;
7. enumera ventanas visibles del sistema;
8. registra miniaturas DWM para las ventanas visibles;
9. calcula el layout inicial y llena el modelo de thumbnails.

Si `start_in_tray = true`, la aplicación termina el arranque escondida en tray.

## Qué deberías ver

En un primer arranque correcto:

- una ventana principal con toolbar superior;
- tarjetas oscuras con una franja de color en la parte superior;
- miniaturas vivas dentro de cada tarjeta;
- icono en el system tray;
- recuento de ventanas visibles y ocultas en la toolbar.

Si no hay ventanas candidatas, la UI muestra un estado vacío.

## Primer recorrido recomendado

1. Pulsa `Tab` para recorrer los layouts.
2. Pulsa `1` a `7` para ver cada layout directamente.
3. Haz click derecho sobre una miniatura para abrir el menú por ventana.
4. Oculta una aplicación del layout y luego restáurala desde tray.
5. Crea una tag desde una aplicación.
6. Abre `Settings` con `O` y revisa filtros, tema y perfiles.
7. Prueba `T` para alternar entre temas.
8. Si usas `Row` o `Column`, navega con rueda o arrastre del botón central.

## Atajos útiles

| Tecla / gesto | Resultado |
| --- | --- |
| `Tab` | siguiente layout |
| `1`…`7` | layout específico |
| `0` | limpiar ratios personalizados del layout actual |
| `R` | refrescar ventanas |
| `A` | alternar animaciones |
| `H` | alternar toolbar |
| `I` | alternar información de ventana |
| `P` | alternar always-on-top |
| `T` | cambiar tema |
| `O` | abrir settings |
| `M` | abrir menú de aplicación |
| `Alt` | alternar toolbar |
| click izquierdo miniatura | activar ventana |
| click derecho miniatura | menú contextual por app/ventana |
| click izquierdo tray | mostrar/ocultar ventana principal |
| click derecho tray | abrir menú rápido |
| `Esc` | salir |

## Archivos que conviene mirar temprano

- `README.md` — resumen general del proyecto.
- `PRD.md` — objetivos de producto actualizados.
- `docs/ARCHITECTURE.md` — arquitectura y diagramas.
- `docs/CONFIGURATION.md` — todas las claves de settings.
- `docs/PROJECT_STRUCTURE.md` — mapa del repositorio.
- `docs/IMPLEMENTATION.md` — detalle técnico por módulo.

## Rutas importantes

### Configuración

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\profiles\<perfil>.toml
```

### Logs

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

### UI y assets

```text
ui/main.slint
assets/themes.json
```

## Desarrollo local

Las comprobaciones más útiles durante trabajo diario son:

```bash
cargo check
cargo test
cargo clippy -- -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

En el workspace ya existen tareas de VS Code para estos comandos.

## Problemas habituales

### La app abre pero no veo miniaturas

Revisa:

- que existan ventanas de usuario visibles;
- que no hayas dejado un filtro activo por monitor, tag o app;
- que DWM esté disponible;
- que Panopticon no esté arrancando oculto en tray.

### El tray desapareció después de reiniciar Explorer

El runtime intenta re-registrarlo automáticamente cuando recibe `TaskbarCreated`.

### Un perfil no parece cargar

Lanza la app con `--profile <nombre>` y verifica si existe `%APPDATA%\Panopticon\profiles\<nombre>.toml`.

### En modo dock algunas acciones se comportan diferente

Es normal: el modo appbar modifica estilo de ventana, topmost y fuerza `hide_on_select` a `false` de forma efectiva.
