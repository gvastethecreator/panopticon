# Product Requirements Document (PRD)

## Proyecto: Visor de Ventanas Multipantalla (Nombre en clave: "Panopticon")

**Versión:** 1.0.0
**Estado:** Listo para Desarrollo
**Plataforma Target:** Windows 10 / Windows 11 (64-bit)
**Lenguaje Principal:** Rust

---

## 1. Visión General del Producto

"Panopticon" es una utilidad de sistema open-source para Windows que proporciona una vista global e interactiva de todas las ventanas abiertas en el escritorio. Utilizando aceleración por hardware nativa, muestra miniaturas (*thumbnails*) en tiempo real de las ventanas activas, organizadas mediante un motor de layouts matemáticos. El objetivo es mejorar el flujo de trabajo de power-users, permitiendo cambiar de contexto visualmente de forma rápida y eficiente.

## 2. Stack Tecnológico Requerido

El agente debe ceñirse estrictamente a este stack para garantizar rendimiento de bajo nivel y seguridad de memoria:

* **Lenguaje:** Rust (Edición 2021 o superior).
* **Interacción con el SO:** Crate `windows` (bindings oficiales de Microsoft).
* **Framework UI:** `egui` (con integración nativa, ej. `eframe`) O ventana nativa Win32 pura con renderizado Direct2D/Direct3D si se requiere mayor control sobre el canal alfa.
* **APIs Críticas de Windows:** Win32 API (User32), Desktop Window Manager (DWM) API.

## 3. Requisitos Funcionales (Core MVP)

### 3.1. Descubrimiento y Filtrado de Ventanas

* **Funcionalidad:** El sistema debe enumerar todas las ventanas de nivel superior (Top-Level Windows).
* **APIs a utilizar:** `EnumWindows`, `GetWindowTextW`, `GetWindowThreadProcessId`, `GetWindowLongW`.
* **Reglas de Filtrado Estrictas:** El agente DEBE filtrar procesos en segundo plano, ventanas invisibles, tooltips y ventanas del sistema (como la barra de tareas o el menú inicio).
  * *Condiciones de validación:* La ventana debe ser visible (`IsWindowVisible`), no ser una ventana de herramientas (`WS_EX_TOOLWINDOW`), y preferiblemente tener un título válido.

### 3.2. Renderizado de Thumbnails en Tiempo Real

* **Funcionalidad:** Mostrar previsualizaciones dinámicas sin capturar bitmaps por software.
* **APIs a utilizar:** `DwmRegisterThumbnail`, `DwmUpdateThumbnailProperties`, `DwmUnregisterThumbnail`.
* **Comportamiento:** * Al mapear una ventana a un área de la UI, se debe registrar el thumbnail vinculando el HWND destino (la ventana abierta) con el HWND origen (la aplicación Panopticon).
  * La propiedad `fVisible` debe manejarse correctamente al cambiar de layouts o páginas.
  * El aspect ratio de la ventana original debe mantenerse (`fSourceClientAreaOnly = true` es recomendado).

### 3.3. Motor de Layouts

* **Funcionalidad:** Organizar dinámicamente los rectángulos de destino (`rcDestination`) de los thumbnails según algoritmos matemáticos en la ventana principal.
* **Modos Soportados:**
    1. **Grid (Cuadrícula):** Distribución equitativa estándar (ej. 3x3, 4x4).
    2. **Mosaic:** Cuadrícula dinámica que ajusta el tamaño basado en el aspect ratio de cada ventana.
    3. **Bento:** Un elemento principal grande (ventana activa recientemente) y elementos más pequeños alrededor.
    4. **Fibonacci (Espiral):** División del espacio siguiendo la proporción áurea.
    5. **Columns:** Cajas organizadas en columnas verticales fluidas (estilo cascada/mampostería).
* **Transiciones:** Las actualizaciones de posición (`DwmUpdateThumbnailProperties`) deben calcularse para redibujar el layout inmediatamente al redimensionar la aplicación.

### 3.4. Interacción y Activación

* **Funcionalidad:** Hacer clic en un área designada a un thumbnail debe traer la ventana correspondiente al frente.
* **APIs a utilizar:** `SetForegroundWindow`, `ShowWindow` (con `SW_RESTORE` si estaba minimizada).
* **Comportamiento:** La aplicación principal debe cerrarse o minimizarse (según configuración) tras activar una ventana destino.

## 4. Requisitos No Funcionales

* **Rendimiento (CPU/RAM):** El uso de CPU en estado de reposo debe ser < 1%. El consumo de RAM no debe exceder los 50MB. Toda la carga gráfica debe delegarse al DWM (GPU).
* **Permisos:** La aplicación debe funcionar en modo usuario (User Mode) sin requerir elevación de privilegios (Run as Administrator), a menos que intente interactuar con ventanas ya elevadas (UIPI - User Interface Privilege Isolation restriction).

## 5. Arquitectura de Estado Sugerida (Para el Agente)

```rust
struct AppState {
    windows: Vec<ManagedWindow>,
    current_layout: LayoutType,
    ui_hwnd: HWND,
}

struct ManagedWindow {
    hwnd: HWND,
    title: String,
    thumbnail_id: isize, // HTHUMBNAIL
    target_rect: Rect,
}

enum LayoutType { Grid, Mosaic, Bento, Fibonacci, Columns }

6. Casos Borde y Manejo de Errores (Edge Cases)

    Ventanas Minimizadas: DWM puede no renderizar thumbnails de ventanas minimizadas dependiendo de la configuración del sistema operativo. El agente debe implementar un fallback (ej. mostrar el icono de la aplicación extrayéndolo con GetClassLongPtrW / GCLP_HICON).

    Ventanas Cerradas Abruptamente: Si el usuario cierra una ventana externamente, la API devolverá error al actualizar el thumbnail. El ciclo de vida de la UI debe manejar el error silenciosamente, limpiar el HTHUMBNAIL obsoleto y forzar un reflow del layout.

    DPI Awareness: La aplicación DEBE ser Per-Monitor DPI Aware (SetProcessDpiAwarenessContext) para calcular correctamente los píxeles de los rectángulos en monitores con distintas escalas (100%, 150%, etc.).

7. Criterios de Aceptación (DoD - Definition of Done)

    El código compila en cargo build --release sin warnings de seguridad.

    La aplicación se lanza y muestra al menos el layout "Grid" con las ventanas actuales.

    Los videos/animaciones dentro de las ventanas monitoreadas se ven en movimiento dentro de los thumbnails.

    Hacer clic en un thumbnail enfoca correctamente la ventana seleccionada.
    El agente no consume más del 1% de CPU en estado de reposo.
    El consumo de RAM no excede los 50MB durante la ejecución normal.
    El código sigue las mejores prácticas de Rust (ownership, borrowing, error handling) y no contiene memory leaks o condiciones de carrera.
    El proyecto se documenta adecuadamente con comentarios y un README.md que explique cómo compilar y usar la aplicación.
