# Política de seguridad

## Cómo reportar una vulnerabilidad

Si encuentras una vulnerabilidad o un problema serio relacionado con:

- ejecución de código,
- manejo inseguro de memoria,
- exposición involuntaria de datos,
- abuso del proceso Win32 o de handles,

por favor **no abras una issue pública primero**.

En su lugar, contacta al mantenedor mediante el canal privado que corresponda en GitHub o abre una security advisory del repositorio si está disponible.

## Qué incluir

- descripción del impacto,
- versión/commit afectado,
- pasos de reproducción,
- workaround si existe,
- prueba de concepto mínima.

## Alcance

El proyecto es una app local de escritorio para Windows. Los reportes más valiosos serán los relacionados con seguridad de memoria, persistencia local, integridad del proceso o interacciones inseguras con APIs Win32.
