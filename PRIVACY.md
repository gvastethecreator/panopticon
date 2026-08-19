# Panopticon Privacy Policy

**Effective date:** August 17, 2026  
**Publisher:** To be completed with the verified Microsoft Store publisher name before submission

Panopticon is a local-first Windows desktop utility that discovers open top-level windows and displays live previews through the Windows Desktop Window Manager (DWM). It provides layouts, filters, grouping, tags, workspaces, tray access, global shortcuts, and optional appbar/dock behavior.

This policy explains what Panopticon accesses, what it stores, and when it uses the network.

## 1. Desktop information Panopticon may access

To provide its core window-management experience, Panopticon may access information exposed by Windows about open desktop windows, including:

- window handles and top-level-window state;
- window titles;
- process and executable information needed to identify an application;
- window class names;
- monitor assignment and screen geometry;
- visibility, minimized, cloaked, and activation state;
- application icons;
- live DWM thumbnail surfaces rendered by Windows;
- information required to activate, minimize, restore, close, move, or arrange a window when the user requests that action.

Panopticon uses these values locally to decide which windows to show and how to display or operate them.

## 2. Live thumbnails

Panopticon registers DWM thumbnails so Windows can render live previews of open windows inside the Panopticon interface.

Panopticon is not designed to upload these previews, use them for advertising, or send them to GVASTETHECREATOR. The previews may display sensitive information already visible in another open application. Users should treat Panopticon's dashboard and screenshots as potentially sensitive and avoid sharing them without review.

The current product is not designed as a screen recorder and does not intentionally create a remote archive of window contents.

## 3. Local storage

Panopticon stores application configuration locally. Depending on enabled features, local data may include:

- layout, theme, color, background, animation, language, and visibility preferences;
- custom keyboard shortcuts;
- filters and per-application rules based on titles, process names, class names, monitor, tags, or other window metadata;
- workspace names and workspace-specific settings;
- saved tags and grouping preferences;
- window-size and appbar/dock configuration;
- local background-image paths selected by the user;
- logs needed to diagnose startup, persistence, DWM, shell, or update-check errors.

Configuration is stored in local TOML files under the user's Windows profile. Panopticon does not require an account.

Because window titles, application names, workspace names, tags, and file paths can contain personal or confidential information, users should review settings, logs, and screenshots before sharing them.

## 4. Network activity

### Microsoft Store build

The Microsoft Store build is compiled with Store-managed updates. It does not start Panopticon's GitHub Releases update request. Package delivery, licensing, crash reporting, and update infrastructure may be provided by Microsoft according to Microsoft's own terms and privacy practices.

### Direct/GitHub build

The direct build may perform a bounded HTTPS request to the public GitHub Releases API to check whether a newer published Panopticon version exists. The request includes a Panopticon version user agent and standard HTTP metadata such as the network address normally visible to the destination service.

The update checker retrieves release metadata only. It is not designed to upload window titles, thumbnails, process lists, settings, workspaces, tags, logs, or user files.

Panopticon may also open project, documentation, release, or support links when the user explicitly chooses them.

## 5. Information Panopticon does not intentionally collect

Panopticon is not designed to collect or transmit:

- account passwords or authentication tokens;
- keystroke contents entered into other applications;
- prompts, responses, conversations, emails, or document contents as structured records;
- contact lists;
- browsing history;
- advertising identifiers;
- the contents of local files other than configuration/background assets explicitly selected for Panopticon;
- window metadata or thumbnails for sale or advertising.

A live thumbnail can visually contain any content shown by its source window. This visual display is part of the requested local feature and should not be confused with a claim that the source content is never visible to Panopticon's process or user interface.

## 6. Sharing and sale of data

GVASTETHECREATOR does not sell Panopticon data.

Panopticon does not intentionally share desktop-window metadata, live thumbnails, workspaces, settings, or tags with GVASTETHECREATOR or third parties.

## 7. Logs, screenshots, and support

Users may voluntarily share logs, screenshots, configuration files, or diagnostic details when requesting support. These materials may contain:

- window titles;
- application/process names;
- local paths;
- workspace or tag names;
- desktop images;
- visible content from open applications.

Review and redact support material before sharing it publicly. Close sensitive windows before creating screenshots or recordings.

## 8. Data retention and deletion

Panopticon keeps local settings, workspaces, tags, rules, and logs until the user removes them or the relevant Windows package/profile data is removed.

Uninstall and upgrade behavior must be tested for each distribution channel. Removing Panopticon does not remove the original files or applications whose windows were displayed. User-selected local background images remain owned and controlled by the user.

## 9. Security and permissions

Panopticon uses Win32, DWM, tray, hotkey, and shell APIs required for desktop operation. It is designed for normal user-level operation and does not require silent elevation for ordinary use.

No software can guarantee absolute security. Report security issues privately according to [`SECURITY.md`](SECURITY.md), without publishing private window contents, credentials, or sensitive logs.

## 10. Children's privacy

Panopticon is a desktop productivity utility and is not directed to children. It does not knowingly collect personal information from children.

## 11. Changes to this policy

This policy may be updated if Panopticon changes its desktop access, storage, telemetry, update, network, or support behavior. Material changes will update this document and its effective date.

## 12. Contact

Privacy and support questions can be submitted through the public project channels without including credentials, private window contents, confidential screenshots, or sensitive local paths.

Repository: `gvastethecreator/panopticon`

---

# Política de Privacidad de Panopticon

**Fecha de vigencia:** 17 de agosto de 2026  
**Publicador:** debe completarse con el nombre verificado de Microsoft Store antes de la submission

Panopticon es una utilidad local para Windows que descubre ventanas superiores abiertas y muestra previews en vivo mediante Desktop Window Manager (DWM). Ofrece layouts, filtros, agrupación, tags, workspaces, bandeja, atajos globales y un modo appbar/dock opcional.

Esta política explica qué consulta Panopticon, qué guarda y cuándo utiliza la red.

## 1. Información del escritorio que puede consultar

Para ofrecer su experiencia principal, Panopticon puede consultar información expuesta por Windows sobre ventanas abiertas, incluyendo:

- handles y estado de ventanas superiores;
- títulos de ventanas;
- información de procesos y ejecutables necesaria para identificar aplicaciones;
- nombres de clase de ventana;
- monitor y geometría de pantalla;
- visibilidad, minimización, cloaking y activación;
- iconos de aplicaciones;
- superficies de thumbnails DWM renderizadas por Windows;
- información necesaria para activar, minimizar, restaurar, cerrar, mover u organizar una ventana cuando el usuario lo solicita.

Panopticon usa estos valores localmente para decidir qué mostrar y cómo operar cada ventana.

## 2. Thumbnails en vivo

Panopticon registra thumbnails DWM para que Windows renderice previews de ventanas abiertas dentro de su interfaz.

Panopticon no está diseñado para subir estas previews, utilizarlas para publicidad ni enviarlas a GVASTETHECREATOR. Las previews pueden mostrar información sensible visible en otra aplicación. El dashboard y sus capturas deben tratarse como material potencialmente sensible.

El producto actual no está diseñado como grabador de pantalla ni crea intencionalmente un archivo remoto del contenido de ventanas.

## 3. Almacenamiento local

Panopticon guarda configuración local. Según las funciones habilitadas, puede incluir:

- layout, tema, color, fondo, animación, idioma y preferencias de visibilidad;
- atajos personalizados;
- filtros y reglas por aplicación basados en título, proceso, clase, monitor, tags u otros metadatos;
- nombres y ajustes de workspaces;
- tags y preferencias de agrupación;
- tamaño de ventana y configuración appbar/dock;
- rutas de imágenes de fondo seleccionadas por el usuario;
- logs para diagnosticar inicio, persistencia, DWM, shell o comprobación de updates.

La configuración se guarda en archivos TOML locales dentro del perfil de Windows. Panopticon no requiere una cuenta.

Los títulos, aplicaciones, workspaces, tags y rutas pueden contener información personal o confidencial. Deben revisarse antes de compartir configuraciones, logs o capturas.

## 4. Actividad de red

### Versión de Microsoft Store

La versión de Microsoft Store se compila con updates administrados por Store. No inicia la consulta de Panopticon a GitHub Releases. Microsoft puede proporcionar distribución, licencias, reportes de fallos y updates según sus propias condiciones y políticas.

### Versión directa/GitHub

La versión directa puede hacer una consulta HTTPS acotada a la API pública de GitHub Releases para comprobar si existe una versión publicada más nueva. La solicitud incluye la versión de Panopticon como user agent y metadatos HTTP normales, como la dirección de red visible para el servicio de destino.

El checker obtiene únicamente metadata de releases. No está diseñado para subir títulos, thumbnails, listas de procesos, ajustes, workspaces, tags, logs ni archivos.

Panopticon también puede abrir enlaces de proyecto, documentación, releases o soporte cuando el usuario los selecciona.

## 5. Información que no recopila intencionalmente

Panopticon no está diseñado para recopilar o transmitir:

- contraseñas o tokens de autenticación;
- contenido de teclas escrito en otras aplicaciones;
- prompts, respuestas, conversaciones, correos o documentos como registros estructurados;
- contactos;
- historial de navegación;
- identificadores publicitarios;
- contenido de archivos locales salvo configuración o fondos seleccionados explícitamente;
- metadatos o thumbnails para venta o publicidad.

Una preview en vivo puede contener visualmente cualquier información mostrada por su ventana de origen. Esa visualización es parte de la función local solicitada y no debe confundirse con una promesa de que el contenido nunca es visible para el proceso o la interfaz.

## 6. Cesión o venta

GVASTETHECREATOR no vende datos de Panopticon.

Panopticon no comparte intencionalmente metadata de ventanas, thumbnails, workspaces, ajustes ni tags con GVASTETHECREATOR o terceros.

## 7. Logs, capturas y soporte

El usuario puede compartir voluntariamente logs, capturas, configuración o diagnósticos. Ese material puede contener:

- títulos de ventanas;
- aplicaciones o procesos;
- rutas locales;
- nombres de workspaces o tags;
- imágenes del escritorio;
- contenido visible de otras aplicaciones.

Debe revisarse y redactarse antes de publicarlo. Se recomienda cerrar ventanas sensibles antes de crear capturas o grabaciones.

## 8. Conservación y eliminación

Panopticon conserva ajustes, workspaces, tags, reglas y logs hasta que el usuario los elimina o se remueven los datos del paquete/perfil.

La desinstalación y actualización deben probarse para cada canal. Remover Panopticon no elimina archivos originales ni las aplicaciones cuyas ventanas se mostraron. Las imágenes de fondo elegidas siguen siendo propiedad y control del usuario.

## 9. Seguridad y permisos

Panopticon utiliza APIs Win32, DWM, bandeja, hotkeys y shell necesarias para escritorio. Está diseñado para operar como usuario normal y no requiere elevación silenciosa para uso ordinario.

Ningún software puede garantizar seguridad absoluta. Los problemas deben reportarse de forma privada según [`SECURITY.md`](SECURITY.md), sin publicar contenido de ventanas, credenciales o logs sensibles.

## 10. Privacidad de menores

Panopticon es una utilidad de productividad y no está dirigida a menores. No recopila conscientemente información personal de menores.

## 11. Cambios

Esta política puede actualizarse si cambian el acceso al escritorio, almacenamiento, telemetría, updates, red o soporte. Los cambios materiales actualizarán el documento y la fecha de vigencia.

## 12. Contacto

Las consultas pueden enviarse por los canales públicos del proyecto sin incluir credenciales, contenido privado de ventanas, capturas confidenciales ni rutas sensibles.

Repositorio: `gvastethecreator/panopticon`
