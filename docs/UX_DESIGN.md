# Diseño UX/UI

Este documento describe cómo está pensada la experiencia de uso de Panopticon, qué superficies visuales la componen y qué principios se perciben en la implementación actual.

## Intención de diseño

Panopticon no se comporta como una aplicación documental o de formularios. Está diseñado como una **herramienta visual de observación y cambio de contexto**. La UI debe permitir que el usuario:

- vea muchas ventanas al mismo tiempo;
- identifique rápidamente qué app es cada tarjeta;
- active o descarte ventanas con pocos gestos;
- mantenga contexto persistente entre sesiones.

## Superficies principales

### 1. Ventana principal (`MainWindow`)

Es el tablero del producto. Contiene:

- toolbar superior opcional;
- área principal con thumbnails;
- estado vacío cuando no hay ventanas;
- overlay scrollbar en layouts con overflow;
- handles de resize para layouts que soportan ratios persistentes.

### 2. Toolbar

La toolbar resume el estado operativo del tablero:

- layout actual;
- número de ventanas visibles;
- número de apps ocultas;
- intervalo de refresco;
- filtros activos y agrupación;
- estado de topmost y animaciones.

Además funciona como superficie interactiva:

- click para ciclar layouts;
- click derecho para abrir el menú principal.

### 3. Tarjetas de miniatura (`ThumbnailCard`)

Cada ventana del sistema se representa como una tarjeta con:

- franja superior de accent;
- placeholder oscuro donde se dibuja encima la miniatura DWM;
- información opcional de título y app;
- icono opcional;
- estados visuales de hover/active;
- estado alternativo para ventana minimizada.

#### Decisiones visibles

- el borde cambia al hacer hover o cuando la tarjeta está activa;
- la tarjeta no renderiza la miniatura por sí misma: deja una zona preparada para que DWM la superponga;
- el diseño prioriza contraste oscuro con accent cálido o derivado del tema.

### 4. Settings window

Es el panel declarativo de configuración. Está organizado por bloques:

- comportamiento;
- display;
- theme;
- layout;
- refresh;
- fixed dimensions;
- dock;
- filters;
- background;
- profiles;
- hidden apps;
- shortcuts.

Su propósito UX es que el usuario pueda ajustar casi todo sin editar TOML manualmente.

### 5. Tag dialog

Es un diálogo compacto para crear una tag y asignarla a una app concreta. Pide:

- nombre de tag;
- color preset;
- confirmación de creación/asignación.

### 6. Tray icon y menús nativos

Panopticon adopta un modelo “tray-first”:

- puede permanecer vivo aunque la ventana principal esté escondida;
- el tray permite reabrir, refrescar, filtrar y restaurar apps ocultas;
- es el centro operativo cuando se usa la app de forma persistente durante toda la sesión.

## Lenguaje visual

### Paleta

El tema clásico usa una base oscura con accent ámbar, pero el sistema de temas permite derivar paletas a partir de `assets/themes.json`.

Elementos cromáticos principales:

- **bg**: fondo general;
- **toolbar-bg**: barra superior;
- **card-bg / panel-bg / surface**: superficies escalonadas;
- **accent**: color protagonista del estado activo o del grupo;
- **muted / label / text**: niveles de jerarquía tipográfica.

### Contraste y profundidad

La UI usa:

- superficies oscuras apiladas;
- bordes finos y discretos;
- franja superior de accent;
- sombras sutiles en overlays;
- un lenguaje visual compatible con Windows 11 cuando `use_system_backdrop` está activo.

### Temas

Los temas no son solo cosméticos: afectan casi todas las superficies y se interpolan con animación, de forma que el cambio no sea brusco.

## Modos de layout y modelo mental

| Layout | Modelo mental | Cuándo resulta más útil |
| --- | --- | --- |
| `Grid` | cuadrícula regular | overview general, muchos elementos similares |
| `Mosaic` | filas con ancho adaptado a ratio | mix de ventanas anchas y altas |
| `Bento` | una ventana protagonista + secundarias | foco principal con contexto lateral |
| `Fibonacci` | división progresiva del espacio | exploración visual y composiciones asimétricas |
| `Columns` | columnas balanceadas | flujos tipo tablero o mampostería |
| `Row` | tira horizontal scrollable | comparación secuencial de muchas ventanas |
| `Column` | tira vertical scrollable | uso en formato panel o dock alto |

## Interacciones principales

### Mouse

- click izquierdo en tarjeta: activa ventana;
- click derecho en tarjeta: abre menú contextual por ventana;
- drag en separadores: ajusta proporciones persistentes;
- rueda / pan con botón central: desplaza layouts con overflow;
- click izquierdo en tray: alterna visibilidad;
- click derecho en tray: abre menú principal.

### Teclado

- `1` a `7`: cambio directo de layout;
- `Tab`: siguiente layout;
- `0`: reset de ratios;
- `R`: refresh;
- `A`, `H`, `I`, `P`, `T`: toggles rápidos;
- `O`: settings;
- `M`: menú principal;
- `Alt`: toolbar;
- `Esc`: salida.

## Estados especiales de la UI

### Ventana minimizada

Cuando la ventana origen está minimizada, la tarjeta no intenta sostener una miniatura útil y cambia a una presentación centrada en el icono y el nombre de la app.

### Estado vacío

Si no hay ventanas elegibles, la UI presenta un panel vacío central con texto explicativo en lugar de dejar un lienzo muerto.

### Scrollbar overlay

En lugar de scrollbars permanentes, Panopticon usa overlays discretos que aparecen con actividad y se desvanecen después. Esto mantiene el tablero limpio cuando no hay overflow.

## Principios UX que se ven reflejados en la implementación

### 1. Prioridad a la observación

El contenido principal son las ventanas del usuario, no los controles de la app. La interfaz intenta ocupar poco “ruido” visual y deja protagonismo a las miniaturas.

### 2. Control inmediato

Casi todas las acciones frecuentes están a un gesto o tecla:

- activar;
- ocultar;
- refrescar;
- cambiar layout;
- filtrar;
- abrir settings.

### 3. Persistencia del contexto

Panopticon recuerda filtros, agrupación, colores, tags, layouts y perfiles. La UX no trata cada arranque como una sesión nueva sin memoria.

### 4. Utilidad de escritorio, no documento

La presencia de tray, dock/appbar, topmost y start-in-tray muestra claramente que la aplicación está pensada para convivir con el escritorio, no para abrirse y cerrarse puntualmente.

## Observaciones de diseño/implementación

- la UI declarativa define overlays de menú, pero el flujo activo principal usa menús nativos Win32;
- la sección de tamaño del settings está orientada al grosor del dock/appbar, no al tamaño libre de la ventana flotante;
- el sistema de tags está bien integrado, pero todavía se apoya mucho en el menú contextual por ventana y menos en edición masiva desde settings.

## Oportunidades de evolución UX

1. hacer más visible el concepto de perfiles y su impacto en el arranque;
2. exponer en UI opciones avanzadas que hoy existen solo en TOML, como el modo de refresco por thumbnail;
3. decidir si los overlays de menú declarativos deben activarse o eliminarse;
4. mejorar la comunicación visual del modo dock y de los filtros activos complejos.
