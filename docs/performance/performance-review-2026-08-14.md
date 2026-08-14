# Revisión de rendimiento — 2026-08-14

## Resumen ejecutivo

El lote reduce trabajo repetido de discovery y DWM, y recorta el binario release. La tasa DWM idle
observada bajó aproximadamente 76%. La CPU mejoró en una medición conservadora, pero su muestreo de
un segundo es cuantizado y ruidoso: no se declara cumplido el objetivo de `<=12 ms/s` o `>=35%`.

Estado: **mejora comprobada, con el gate CPU aún abierto**.

## Alcance y método

- Repositorio: `X:\panopticon`.
- Baseline y final: mismo escritorio Windows, binario release, `RUST_LOG=trace`, nueve muestras de un
  segundo y counters de 60 ticks.
- Superficies: loop UI, DWM, enumeración/catálogo, actualización de labels y features Slint.
- Fuera de alcance aprobado: PERF-05, PERF-07 y PERF-08.

## Métricas

| Métrica | Baseline | Final | Resultado |
| --- | ---: | ---: | --- |
| Tasa DWM idle | ~16.1 sync/s | ~3.9 sync/s | ~76% menos |
| CPU idle, mediana conservadora | 18.75 ms CPU/s | 15.41 ms CPU/s | ~17.8% menos; ruidoso |
| Binario release | 33,626,624 B | 31,634,432 B | 1,992,192 B / 5.92% menos |
| Ventanas propias/sistema en grid | 2 observadas | 0 | Eliminadas |

La ejecución final también produjo medianas de `0 ms/s` por la resolución del contador; se conserva
15.41 ms/s como lectura final conservadora. El rango observado fue 0–139 ms/s durante cambios del
escritorio, por lo que CPU no es una señal estable aislada.

## Hallazgos

| ID | Severidad | Estado | Evidencia e impacto |
| --- | --- | --- | --- |
| PERF-01 | Media | Parcial | Timer adaptativo 16/32/64 ms; menos wakeups, gate CPU no cruzado. |
| PERF-02 | Alta | Cerrado | Cadencia DWM por tiempo: 7–8 sync por ~1.94 s idle frente a 15–16 por ~0.96 s. |
| PERF-03 | Media | Cerrado | Cache de metadata HWND/PID conserva títulos/monitor frescos. |
| PERF-04 | Media | Cerrado | Labels mutan la proyección directa sin clonar/persistir settings completos. |
| PERF-06 | Media | Cerrado | Paleta, tray y Settings consumen `WindowCatalog` canónico. |
| PERF-09 | Media | Cerrado | Features Slint explícitas; release 5.92% menor. |
| PERF-10 | Alta | Cerrado | PID propio y TextInputHost se excluyen antes de DWM/icon/model work. |

## Decisiones y trade-offs

- Se mantuvo 16 ms cuando hay trabajo activo; el ahorro solo entra en estados visibles idle u ocultos.
- La cadencia DWM conserva refresh inmediato cuando cambia el modelo y baja frecuencia solo en idle.
- No se redujo resolución de iconos ni se reescribió reconcile sin un benchmark que cruce sus gates.

## Plan priorizado

1. Si CPU vuelve a ser objetivo, medir ETW/WPA o un contador de mayor duración y carga estable.
2. Perfilar reconcile con 200 ventanas antes de PERF-05.
3. Evaluar iconos 96/128/256 solo con matriz DPI y aprobación visual.

## Verificación

- `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic`: PASS.
- `cargo test --all-targets --locked`: PASS, 170 tests.
- `cargo build --release --locked`: PASS.
- Runtime release: dashboard real, UIA y trace idle; evidencia en
  `C:\Users\cristian\.codex\visualizations\2026\08\14\01a0019c-1352-7b01-9d26-ab5c43bdb4d2\final15-release`.

## Riesgos residuales

- CPU depende del desktop activo y la cuantización del scheduler; no sirve como benchmark reproducible
  sin un workload fijado.
- Tray, appbar/dock y reinicio de Explorer mantienen gate manual.
- PERF-05/07/08 siguen diferidos por decisión aprobada.
