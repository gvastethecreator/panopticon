# Product Requirements Document (PRD)

## Proyecto: Panopticon

**Versión del producto documentado:** 2.0.0  
**Estado del documento:** actualizado para reflejar la implementación actual  
**Plataforma objetivo:** Windows 10 / Windows 11 (64-bit)  
**Stack real:** Rust + Slint + Win32/DWM  
**Naturaleza del producto:** utilidad local de escritorio; sin backend ni servicios remotos

---

## 1. Resumen ejecutivo

Panopticon es una utilidad visual para Windows que ofrece una visión consolidada de las ventanas abiertas del escritorio. La aplicación enumera ventanas de nivel superior, crea miniaturas vivas mediante la API de Desktop Window Manager (DWM) y las presenta dentro de un tablero interactivo con distintos algoritmos de layout.

El objetivo del producto es permitir que un usuario cambie de contexto, organice ventanas y recupere foco visual sobre su espacio de trabajo de forma más rápida que con `Alt+Tab` o con la barra de tareas tradicional.

---

## 2. Problema que resuelve

Los usuarios intensivos de escritorio suelen tener muchas ventanas abiertas al mismo tiempo. Los mecanismos estándar del sistema operativo no siempre resuelven bien estos escenarios:

- la barra de tareas ofrece contexto limitado;
- `Alt+Tab` es secuencial y no siempre escalable;
- agrupar ventanas por monitor, aplicación o tarea requiere varios pasos manuales;
- distinguir rápidamente ventanas minimizadas, procesos ocultos o agrupaciones manuales es costoso.

Panopticon resuelve esto con una vista persistente y configurable del estado del escritorio.

---

## 3. Usuarios objetivo

### 3.1. Usuario principal

- personas que trabajan con muchas ventanas simultáneamente;
- usuarios de setups multi-monitor;
- perfiles técnicos, productivos o creativos que necesitan supervisar varias apps a la vez.

### 3.2. Usuario secundario

- desarrolladores que quieren agrupar flujos de trabajo por proyecto;
- operadores o analistas que necesitan mantener un tablero visual del escritorio;
- power users que prefieren utilidades de tray, perfiles y automatización ligera.

---

## 4. Objetivos del producto

### 4.1. Objetivos funcionales

1. Mostrar miniaturas vivas de ventanas abiertas sin capturas bitmap manuales.
2. Permitir reordenar visualmente las ventanas mediante layouts adaptativos.
3. Facilitar activación, ocultado, filtrado y agrupación desde una sola superficie.
4. Persistir preferencias y reglas por aplicación entre sesiones.
5. Mantener una UX coherente con Windows mediante tray icon, menús nativos, appbar y soporte DPI.

### 4.2. Objetivos técnicos

1. Mantener la mayor parte del pipeline gráfico delegado a DWM.
2. Concentrar el cálculo de layouts en lógica pura y testeable.
3. Aislar la interoperabilidad Win32/FFI en bloques `unsafe` acotados y comentados.
4. Permitir personalización sin depender de infraestructura externa.

### 4.3. No objetivos actuales

- soporte multiplataforma real fuera de Windows;
- sincronización en la nube o configuración remota;
- automatización basada en reglas avanzadas o scripting;
- analítica, telemetría o backend SaaS;
- capturas persistentes, grabación de vídeo o historial temporal de ventanas.

---

## 5. Alcance funcional actual

### 5.1. Descubrimiento de ventanas

El producto debe:

- enumerar ventanas de nivel superior con `EnumWindows`;
- ignorar ventanas invisibles, tool windows, ventanas no activables sin `WS_EX_APPWINDOW`, ventanas con owner no relevante y superficies del sistema conocidas;
- capturar para cada ventana: `HWND`, título, clase, ruta del ejecutable, nombre de proceso, identificador persistente (`app_id`) y monitor.

### 5.2. Visualización de thumbnails

El producto debe:

- registrar una miniatura DWM por ventana visible gestionada;
- actualizar rectángulos de destino durante resize, animación, scroll y cambio de layout;
- liberar la miniatura cuando la ventana ya no aplica o cuando el origen está minimizado;
- usar un placeholder visual e icono de aplicación cuando la ventana fuente está minimizada.

### 5.3. Layouts

El producto debe soportar estos layouts:

1. `Grid`
2. `Mosaic`
3. `Bento`
4. `Fibonacci`
5. `Columns`
6. `Row`
7. `Column`

Además, debe:

- permitir cambiar entre layouts por teclado, toolbar o tray;
- guardar el layout inicial preferido;
- persistir ratios personalizados cuando el usuario arrastra separadores;
- soportar overflow con scroll horizontal o vertical en `Row` y `Column`.

### 5.4. Interacción con ventanas

El producto debe:

- activar una ventana al hacer click en su miniatura;
- restaurar una ventana minimizada antes de enfocarla;
- permitir cerrar la ventana objetivo o terminar su proceso desde el menú contextual;
- permitir ocultar una aplicación del layout sin cerrar el proceso real.

### 5.5. Filtros, tags y agrupación

El producto debe:

- filtrar por monitor;
- filtrar por tag manual;
- filtrar por aplicación (`app_id`);
- agrupar visualmente el orden de ventanas por aplicación, monitor, título o clase;
- permitir crear y asignar tags manuales desde la UI;
- asociar colores a tags y a aplicaciones concretas.

### 5.6. Persistencia y perfiles

El producto debe:

- guardar configuración global y reglas por app en TOML;
- soportar perfiles múltiples mediante `--profile <nombre>`;
- permitir guardar un perfil desde la ventana de settings;
- permitir abrir otra instancia usando otro perfil;
- sembrar perfiles por defecto si no existe ninguno adicional.

### 5.7. Utilidad de tray y ciclo de vida

El producto debe:

- registrar icono en el system tray;
- restaurarlo si Explorer reinicia (`TaskbarCreated`);
- permitir minimizar o cerrar hacia tray según configuración;
- iniciar oculto en tray si el usuario lo configura;
- salir limpiamente liberando thumbnails y eliminando el icono del tray.

### 5.8. Personalización visual

El producto debe:

- aplicar tema clásico o presets derivados de `assets/themes.json`;
- animar transiciones de tema;
- permitir fondo sólido y fondo por imagen detrás del tablero;
- usar esquinas redondeadas y backdrop de Windows 11 cuando esté disponible.

### 5.9. Dock / appbar

El producto puede acoplarse a un borde de pantalla como appbar. En ese modo debe:

- reservar espacio de escritorio con `SHAppBarMessage`;
- recolocarse ante cambios del shell o display;
- desactivar efectivamente `hide_on_select`;
- bloquear ciertos comandos del menú del sistema asociados a mover o cerrar la ventana.

---

## 6. Requisitos no funcionales

### 6.1. Plataforma

- soporte exclusivo para Windows 10/11;
- dependencia explícita de DWM y Win32;
- sin requerir privilegios de administrador para el caso general.

### 6.2. Rendimiento

- la composición gráfica debe delegarse a DWM siempre que sea posible;
- el refresco de enumeración debe ser configurable;
- las animaciones deben ser suaves y acotadas en tiempo (`180 ms` para layouts, `220 ms` para temas);
- el refresco del UI loop debe poder convivir con el event loop principal sin bloquear la aplicación.

### 6.3. Robustez

- si una ventana desaparece, la miniatura asociada debe eliminarse sin tumbar la app;
- si un thumbnail falla al actualizarse, debe regenerarse o soltarse con degradación controlada;
- si un icono no puede generarse, debe existir fallback al icono del sistema o del ejecutable.

### 6.4. Seguridad y mantenibilidad

- el código `unsafe` debe estar justificado con comentarios `SAFETY`;
- no deben exponerse raw pointers en APIs públicas del crate;
- el motor de layouts y la normalización de settings deben seguir siendo fácilmente testeables.

### 6.5. Observabilidad

- la app debe emitir logs estructurados a archivo local;
- el path de logs debe ser determinista y fácil de inspeccionar durante soporte o debugging.

---

## 7. Restricciones y dependencias

### 7.1. Dependencias de runtime

- `slint` para la UI declarativa;
- crate `windows` para DWM, User32, Shell, GDI, HiDPI y Threading;
- `raw-window-handle` para obtener el `HWND` de la ventana Slint;
- `serde`, `serde_json` y `toml` para persistencia y catálogo de temas;
- `tracing` y `tracing-appender` para logging.

### 7.2. Restricciones técnicas relevantes

- el proyecto usa un event loop single-threaded compartido con Slint/Win32;
- la interacción con ventanas elevadas puede verse limitada por UIPI;
- algunas capacidades dependen del comportamiento concreto de DWM en el sistema del usuario;

---

## 8. Casos borde importantes

1. **Ventana minimizada**  
    El thumbnail DWM puede dejar de ser útil; Panopticon libera el thumbnail y muestra fallback visual basado en icono.

2. **Ventana cerrada o proceso terminado**  
    La siguiente enumeración o actualización elimina el elemento y refluye el layout.

3. **Explorer reiniciado**  
    El icono del tray debe registrarse de nuevo al recibir `TaskbarCreated`.

4. **Monitores con distinto DPI**  
    La app debe ejecutarse con `PER_MONITOR_AWARE_V2` y recalcular rectángulos con el factor de escala correcto.

5. **Layouts scrollables**  
    `Row` y `Column` pueden exceder el viewport; la UI debe ofrecer desplazamiento y overlay scrollbar.

6. **Modo dock**  
    La ventana cambia de rol visual y de restricciones del sistema; algunas opciones dejan de comportarse igual que en modo flotante.

---

## 9. Criterios de aceptación

El producto se considera funcionalmente aceptable cuando:

1. compila y arranca en Windows con `cargo run` o `cargo run --release`;
2. enumera y muestra miniaturas de ventanas activas en al menos un layout;
3. permite activar una ventana mediante click en miniatura;
4. permite cambiar layout y mantener el tablero consistente al refrescar;
5. persiste settings y reglas por app entre sesiones;
6. soporta tray icon, filtros y menú contextual por ventana;
7. mantiene documentación técnica suficiente para reproducir, mantener y extender el proyecto.

---

## 10. Próximas oportunidades

- ampliar cobertura de tests para Win32/DWM/tray;
- limpiar o retirar componentes de UI declarativa no conectados si ya no forman parte del runtime activo;
- documentar mejor métricas de rendimiento reales en Windows;
