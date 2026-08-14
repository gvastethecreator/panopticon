# Revisión de experiencia de usuario — 2026-08-14

## Resumen ejecutivo

La aplicación real ya no muestra superficies propias/sistema en el grid; sus controles custom,
thumbnails y paleta son accionables por UI Automation y teclado. Settings y comandos se ven en español
sin los cruces detectados, y las acciones riesgosas/recoverables tienen confirmación o salida visible.

Estado: **flujo principal validado en desktop real; quedan escenarios de fallo seguros sin provocar**.

## Alcance y método

- Inspección `PrintWindow` y UI Automation del release en Windows.
- Recorrido real del dashboard, seis páginas de Settings, paleta, selección con flechas y reset.
- Matriz de copy crítico EN/ES por test; inspección visual en español.
- No se ejecutó kill real ni se forzó corrupción/pérdida de datos.

## Evidencia antes/después

| Señal | Antes | Después |
| --- | --- | --- |
| Grid | Incluía Settings y TextInputHost | Ninguna superficie propia/sistema observada |
| Main UIA | Custom controls sin acción suficiente | 59 elementos semánticos, 29 enfocables en el run final; controles con nombre |
| Settings UIA | Text/Image no enfocables | 53 enfocables; nav buttons y 11 checkboxes con nombre |
| Paleta UIA | Solo editor enfocable | 43 enfocables; filas `ListItem`, botones y nombres |
| Paleta teclado | Sin selección Up/Down | Down movió highlight; Home/End y scroll implementados |
| Layout de Settings | Atajos comprimidos, vacíos rígidos y anchos inconsistentes | Retícula de dos columnas, estados vacíos compactos y controles alineados |
| Workspaces | Resumen `[?]`, `default` duplicado y timestamps internos | Resumen correcto y fechas UTC legibles |
| Reset | Mutación inmediata | Confirmación visible; “No” enfocado por defecto; Esc cancela |
| Save failure | Solo logs | Banner + Reintentar guardado + Abrir logs |
| Update check | Espera sin límites claros | 10 s por fase, HTTP 200, body 1 MiB, Cancel |

Capturas finales: `C:\Users\cristian\.codex\visualizations\2026\08\14\01a0019c-1352-7b01-9d26-ab5c43bdb4d2\final15-release`.

## Hallazgos

| ID | Estado | Resultado |
| --- | --- | --- |
| UX-01 | Cerrado | Eligibility estable por PID/process; evidencia real sin self/TextInputHost. |
| UX-02 | Cerrado | Roles, nombres, valores, checked/enabled e invoke en primitives custom. |
| UX-03 | Cerrado | Focus visible, Tab, Space/Enter y flechas en controles compartidos. |
| UX-04 | Cerrado | Thumbnail y cerrar tienen nombre, rol y acción UIA/teclado. |
| UX-05 | Cerrado para copy crítico | Seis páginas españolas distintas + paleta; test EN/ES. |
| UX-06 | Cerrado | List semantics, Up/Down/Home/End y scroll-to-selection. |
| UX-07 | Cerrado en UI/contrato | Confirmación y resultado tipado; kill real no ejecutado. |
| UX-08 | Cerrado | Confirmación de reset observada con “No” predeterminado. |
| UX-09 | Implementado; fallo real no provocado | Banner persistente y acciones de recuperación probadas estáticamente. |
| UX-10 | Cerrado | Timeout/status/body cap/cancel con tests de contrato. |
| UX-11 | Cerrado | Espaciado, distribución y alineación revisados en las seis páginas; Filtros, Atajos, Workspaces y presets corregidos. |

## Decisiones y trade-offs

- Se usaron roles nativos Slint/UIA y foco visible, sin añadir un sistema paralelo de controles.
- El diálogo destructivo usa `MessageBoxW` con `MB_DEFBUTTON2` para hacer segura la tecla Enter.
- No se mató un proceso ni se volvió AppData no escribible: son pruebas con riesgo real para la sesión.

## Plan priorizado

1. En una sesión de QA aislada, probar kill con proceso desechable y access denied/stale PID.
2. Probar persistencia con `APPDATA` redirigido a fixture no escribible, sin tocar datos reales.
3. Hacer un pase Narrator humano de las seis páginas y combos desplegados.

## Verificación

- Matriz final: `main-final.png`, seis `settings-*.png`, `command-palette-final.png`,
  `command-palette-keyboard.png`, `reset-confirmation.png`.
- Test de completitud de claves críticas EN/ES: PASS.
- 170 tests, Clippy pedantic, release build: PASS.

## Riesgos residuales

- UIA confirma semántica y acciones, pero no sustituye una evaluación humana con Narrator.
- El fallo de persistencia y los outcomes de kill no se provocaron en el desktop del usuario.
- Tray/appbar/Explorer siguen fuera de esta matriz visual.
