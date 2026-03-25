//! Modeless settings window and tag-creation dialog for the Panopticon UI.

use std::ffi::c_void;
use std::mem;
use std::sync::Once;

use anyhow::{anyhow, Result};
use panopticon::layout::LayoutType;
use panopticon::settings::{AppSettings, DockEdge};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_OPTIONS_APPLY: u32 = WM_APP + 20;
pub const WM_OPTIONS_CLOSED: u32 = WM_APP + 21;
pub const WM_TAG_CREATED: u32 = WM_APP + 22;

const OPTIONS_CLASS_NAME: PCWSTR = w!("PanopticonOptionsWindow");
const TAG_DIALOG_CLASS_NAME: PCWSTR = w!("PanopticonTagDialogWindow");

const BG_PRESETS: [(&str, &str); 6] = [
    ("Graphite", "181513"),
    ("Slate", "1B2430"),
    ("Ocean", "102A43"),
    ("Forest", "16302B"),
    ("Plum", "2D1E2F"),
    ("Amber", "2B2117"),
];

const TAG_PRESETS: [(&str, &str); 6] = [
    ("Amber", "D29A5C"),
    ("Sky", "5CA9FF"),
    ("Mint", "3CCF91"),
    ("Rose", "FF6B8A"),
    ("Violet", "9B7BFF"),
    ("Sun", "F4B740"),
];

const IDC_OPTIONS_APPLY: i32 = 100;
const IDC_OPTIONS_CLOSE: i32 = 101;
const IDC_ALWAYS_ON_TOP: i32 = 110;
const IDC_ANIMATE: i32 = 111;
const IDC_MINIMIZE_TO_TRAY: i32 = 112;
const IDC_CLOSE_TO_TRAY: i32 = 113;
const IDC_DEFAULT_ASPECT: i32 = 114;
const IDC_DEFAULT_HIDE_ON_SELECT: i32 = 115;
const IDC_SHOW_HEADER: i32 = 116;
const IDC_SHOW_INFO: i32 = 117;
const IDC_USE_SYSTEM_BACKDROP: i32 = 118;
const IDC_REFRESH_1S: i32 = 130;
const IDC_REFRESH_2S: i32 = 131;
const IDC_REFRESH_5S: i32 = 132;
const IDC_REFRESH_10S: i32 = 133;
const IDC_LAYOUT_GRID: i32 = 150;
const IDC_LAYOUT_MOSAIC: i32 = 151;
const IDC_LAYOUT_BENTO: i32 = 152;
const IDC_LAYOUT_FIBONACCI: i32 = 153;
const IDC_LAYOUT_COLUMNS: i32 = 154;
const IDC_LAYOUT_ROW: i32 = 155;
const IDC_LAYOUT_COLUMN: i32 = 156;
const IDC_DOCK_NONE: i32 = 180;
const IDC_DOCK_LEFT: i32 = 181;
const IDC_DOCK_RIGHT: i32 = 182;
const IDC_DOCK_TOP: i32 = 183;
const IDC_DOCK_BOTTOM: i32 = 184;
const IDC_FIXED_WIDTH: i32 = 200;
const IDC_FIXED_HEIGHT: i32 = 201;
const IDC_BG_PRESET_BASE: i32 = 220;

const IDC_TAG_NAME: i32 = 300;
const IDC_TAG_CREATE: i32 = 301;
const IDC_TAG_CANCEL: i32 = 302;
const IDC_TAG_PRESET_BASE: i32 = 320;

pub struct OptionsSubmit {
    pub settings: AppSettings,
}

pub struct TagCreateSubmit {
    pub app_id: String,
    pub display_name: String,
    pub tag_name: String,
    pub color_hex: String,
}

struct OptionsWindowState {
    parent: HWND,
    settings: AppSettings,
}

struct TagDialogState {
    parent: HWND,
    app_id: String,
    display_name: String,
}

pub fn open_options_window(parent: HWND, settings: &AppSettings) -> Result<HWND> {
    register_options_class();

    let instance = unsafe { GetModuleHandleW(None) }.map_err(|_| anyhow!("GetModuleHandleW failed"))?;
    let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
    let state = Box::new(OptionsWindowState {
        parent,
        settings: settings.clone(),
    });

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            OPTIONS_CLASS_NAME,
            w!("Panopticon — Configuración"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            760,
            parent,
            None,
            hinstance,
            Some(Box::into_raw(state).cast::<c_void>()),
        )
    }
    .map_err(|_| anyhow!("CreateWindowExW failed for options window"))?;

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }

    Ok(hwnd)
}

pub fn open_tag_dialog(
    parent: HWND,
    app_id: &str,
    display_name: &str,
    suggested_tag: &str,
    suggested_color_hex: &str,
) -> Result<HWND> {
    register_tag_dialog_class();

    let instance = unsafe { GetModuleHandleW(None) }.map_err(|_| anyhow!("GetModuleHandleW failed"))?;
    let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
    let state = Box::new(TagDialogState {
        parent,
        app_id: app_id.to_owned(),
        display_name: display_name.to_owned(),
    });

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            TAG_DIALOG_CLASS_NAME,
            w!("Crear tag"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            420,
            300,
            parent,
            None,
            hinstance,
            Some(Box::into_raw(state).cast::<c_void>()),
        )
    }
    .map_err(|_| anyhow!("CreateWindowExW failed for tag dialog"))?;

    unsafe {
        create_label(
            hwnd,
            &format!("Aplicación: {display_name}"),
            20,
            16,
            360,
            20,
            0,
        );
        create_label(hwnd, "Nombre del tag", 20, 50, 120, 20, 0);
        create_edit(hwnd, IDC_TAG_NAME, suggested_tag, 20, 74, 360, 26, false);
        create_label(hwnd, "Color del tag", 20, 112, 120, 20, 0);

        let mut x = 20;
        for (index, (label, color_hex)) in TAG_PRESETS.iter().enumerate() {
            let id = IDC_TAG_PRESET_BASE + index as i32;
            create_radio(hwnd, id, label, x, 138, 90, 22, 0, *color_hex == suggested_color_hex);
            x += 96;
        }

        create_button(hwnd, IDC_TAG_CREATE, "Crear y asignar", 180, 208, 120, 30);
        create_button(hwnd, IDC_TAG_CANCEL, "Cancelar", 310, 208, 80, 30);

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }

    Ok(hwnd)
}

unsafe extern "system" fn options_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            if create_struct.is_null() {
                return LRESULT(0);
            }

            let state_ptr = (*create_struct).lpCreateParams.cast::<OptionsWindowState>();
            if state_ptr.is_null() {
                return LRESULT(0);
            }

            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            LRESULT(1)
        }
        WM_CREATE => {
            let Some(state) = options_state(hwnd) else {
                return LRESULT(0);
            };
            populate_options_window(hwnd, &state.settings);
            LRESULT(0)
        }
        WM_COMMAND => {
            let command_id = low_word(wparam.0 as u32) as i32;
            match command_id {
                IDC_OPTIONS_APPLY => {
                    if let Some(state) = options_state(hwnd) {
                        let settings = collect_options(hwnd, &state.settings);
                        let payload = Box::new(OptionsSubmit { settings });
                        let _ = PostMessageW(
                            state.parent,
                            WM_OPTIONS_APPLY,
                            WPARAM(0),
                            LPARAM(Box::into_raw(payload) as isize),
                        );
                    }
                    LRESULT(0)
                }
                IDC_OPTIONS_CLOSE => {
                    let _ = DestroyWindow(hwnd);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = options_state(hwnd) {
                let _ = PostMessageW(state.parent, WM_OPTIONS_CLOSED, WPARAM(0), LPARAM(0));
            }
            let state_ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut OptionsWindowState;
            if !state_ptr.is_null() {
                drop(Box::from_raw(state_ptr));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn tag_dialog_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            if create_struct.is_null() {
                return LRESULT(0);
            }

            let state_ptr = (*create_struct).lpCreateParams.cast::<TagDialogState>();
            if state_ptr.is_null() {
                return LRESULT(0);
            }

            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            LRESULT(1)
        }
        WM_COMMAND => {
            let command_id = low_word(wparam.0 as u32) as i32;
            match command_id {
                IDC_TAG_CREATE => {
                    let Some(state) = tag_dialog_state(hwnd) else {
                        return LRESULT(0);
                    };

                    let tag_name = read_window_text(GetDlgItem(hwnd, IDC_TAG_NAME).unwrap_or_default());
                    if tag_name.trim().is_empty() {
                        let _ = MessageBoxW(
                            hwnd,
                            w!("Escribe un nombre para el tag."),
                            w!("Panopticon"),
                            MB_ICONWARNING | MB_OK,
                        );
                        return LRESULT(0);
                    }

                    let color_hex = selected_tag_color(hwnd).to_owned();
                    let payload = Box::new(TagCreateSubmit {
                        app_id: state.app_id.clone(),
                        display_name: state.display_name.clone(),
                        tag_name,
                        color_hex,
                    });
                    let _ = PostMessageW(
                        state.parent,
                        WM_TAG_CREATED,
                        WPARAM(0),
                        LPARAM(Box::into_raw(payload) as isize),
                    );
                    let _ = DestroyWindow(hwnd);
                    LRESULT(0)
                }
                IDC_TAG_CANCEL => {
                    let _ = DestroyWindow(hwnd);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut TagDialogState;
            if !state_ptr.is_null() {
                drop(Box::from_raw(state_ptr));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn populate_options_window(hwnd: HWND, settings: &AppSettings) {
    unsafe {
        create_group_box(hwnd, "Comportamiento", 16, 16, 280, 170);
        create_checkbox(hwnd, IDC_ALWAYS_ON_TOP, "Siempre visible", 32, 44, 220, 20, settings.always_on_top);
        create_checkbox(hwnd, IDC_ANIMATE, "Animaciones", 32, 68, 220, 20, settings.animate_transitions);
        create_checkbox(hwnd, IDC_MINIMIZE_TO_TRAY, "Ocultar al minimizar", 32, 92, 220, 20, settings.minimize_to_tray);
        create_checkbox(hwnd, IDC_CLOSE_TO_TRAY, "Ocultar al cerrar", 32, 116, 220, 20, settings.close_to_tray);
        create_checkbox(hwnd, IDC_DEFAULT_ASPECT, "Respetar aspect ratio por defecto", 32, 140, 240, 20, settings.preserve_aspect_ratio);
        create_checkbox(hwnd, IDC_DEFAULT_HIDE_ON_SELECT, "Ocultar tras activar ventana", 32, 164, 240, 20, settings.hide_on_select);

        create_group_box(hwnd, "Interfaz", 308, 16, 292, 170);
        create_checkbox(hwnd, IDC_SHOW_HEADER, "Mostrar header", 324, 44, 220, 20, settings.show_toolbar);
        create_checkbox(hwnd, IDC_SHOW_INFO, "Mostrar información bajo thumbnails", 324, 68, 240, 20, settings.show_window_info);
        create_checkbox(hwnd, IDC_USE_SYSTEM_BACKDROP, "Adaptar apariencia a Windows 11", 324, 92, 240, 20, settings.use_system_backdrop);
        create_label(hwnd, "Fondo", 324, 124, 160, 20, 0);
        let mut x = 324;
        for (index, (label, color_hex)) in BG_PRESETS.iter().enumerate() {
            create_radio(
                hwnd,
                IDC_BG_PRESET_BASE + index as i32,
                label,
                x,
                148,
                86,
                20,
                0,
                settings.background_color_hex.eq_ignore_ascii_case(color_hex),
            );
            x += 90;
        }

        create_group_box(hwnd, "Refresh y layout inicial", 16, 196, 280, 250);
        create_label(hwnd, "Refresh", 32, 224, 120, 20, 0);
        create_radio(hwnd, IDC_REFRESH_1S, "1s", 32, 248, 60, 20, 0, settings.refresh_interval_ms == 1_000);
        create_radio(hwnd, IDC_REFRESH_2S, "2s", 92, 248, 60, 20, 0, settings.refresh_interval_ms == 2_000);
        create_radio(hwnd, IDC_REFRESH_5S, "5s", 152, 248, 60, 20, 0, settings.refresh_interval_ms == 5_000);
        create_radio(hwnd, IDC_REFRESH_10S, "10s", 212, 248, 60, 20, 0, settings.refresh_interval_ms == 10_000);

        create_label(hwnd, "Layout", 32, 284, 120, 20, 0);
        create_radio(hwnd, IDC_LAYOUT_GRID, "Grid", 32, 308, 80, 20, 0, settings.initial_layout == LayoutType::Grid);
        create_radio(hwnd, IDC_LAYOUT_MOSAIC, "Mosaic", 112, 308, 80, 20, 0, settings.initial_layout == LayoutType::Mosaic);
        create_radio(hwnd, IDC_LAYOUT_BENTO, "Bento", 192, 308, 80, 20, 0, settings.initial_layout == LayoutType::Bento);
        create_radio(hwnd, IDC_LAYOUT_FIBONACCI, "Fibonacci", 32, 332, 90, 20, 0, settings.initial_layout == LayoutType::Fibonacci);
        create_radio(hwnd, IDC_LAYOUT_COLUMNS, "Columns", 122, 332, 90, 20, 0, settings.initial_layout == LayoutType::Columns);
        create_radio(hwnd, IDC_LAYOUT_ROW, "Row (horizontal)", 32, 356, 120, 20, 0, settings.initial_layout == LayoutType::Row);
        create_radio(hwnd, IDC_LAYOUT_COLUMN, "Column (vertical)", 152, 356, 120, 20, 0, settings.initial_layout == LayoutType::Column);

        create_group_box(hwnd, "Dock y tamaño", 308, 196, 292, 250);
        create_label(hwnd, "Dock", 324, 224, 80, 20, 0);
        create_radio(hwnd, IDC_DOCK_NONE, "Flotante", 324, 248, 80, 20, 0, settings.dock_edge.is_none());
        create_radio(hwnd, IDC_DOCK_LEFT, "Left", 404, 248, 60, 20, 0, settings.dock_edge == Some(DockEdge::Left));
        create_radio(hwnd, IDC_DOCK_RIGHT, "Right", 474, 248, 70, 20, 0, settings.dock_edge == Some(DockEdge::Right));
        create_radio(hwnd, IDC_DOCK_TOP, "Top", 324, 272, 60, 20, 0, settings.dock_edge == Some(DockEdge::Top));
        create_radio(hwnd, IDC_DOCK_BOTTOM, "Bottom", 404, 272, 80, 20, 0, settings.dock_edge == Some(DockEdge::Bottom));

        create_label(hwnd, "Fixed width", 324, 316, 90, 20, 0);
        create_edit(hwnd, IDC_FIXED_WIDTH, &settings.fixed_width.map_or_else(String::new, |value| value.to_string()), 420, 312, 72, 24, true);
        create_label(hwnd, "Fixed height", 324, 348, 90, 20, 0);
        create_edit(hwnd, IDC_FIXED_HEIGHT, &settings.fixed_height.map_or_else(String::new, |value| value.to_string()), 420, 344, 72, 24, true);

        create_group_box(hwnd, "Hotkeys", 16, 458, 584, 180);
        create_label(
            hwnd,
            "Tab: siguiente layout · 1-7: layout directo · R: refresh · A: animaciones\r\nH: header · I: info thumbnails · P: siempre visible · O: abrir configuración\r\nEsc: cerrar app",
            32,
            486,
            540,
            54,
            0,
        );
        create_label(
            hwnd,
            "Tip: el scrollbar aparece al pasar el mouse sólo si realmente hay contenido para desplazar.",
            32,
            550,
            540,
            20,
            0,
        );

        create_button(hwnd, IDC_OPTIONS_APPLY, "Aplicar", 424, 664, 82, 30);
        create_button(hwnd, IDC_OPTIONS_CLOSE, "Cerrar", 516, 664, 82, 30);
    }
}

fn collect_options(hwnd: HWND, base: &AppSettings) -> AppSettings {
    let mut settings = base.clone();

    settings.always_on_top = is_checked(hwnd, IDC_ALWAYS_ON_TOP);
    settings.animate_transitions = is_checked(hwnd, IDC_ANIMATE);
    settings.minimize_to_tray = is_checked(hwnd, IDC_MINIMIZE_TO_TRAY);
    settings.close_to_tray = is_checked(hwnd, IDC_CLOSE_TO_TRAY);
    settings.preserve_aspect_ratio = is_checked(hwnd, IDC_DEFAULT_ASPECT);
    settings.hide_on_select = is_checked(hwnd, IDC_DEFAULT_HIDE_ON_SELECT);
    settings.show_toolbar = is_checked(hwnd, IDC_SHOW_HEADER);
    settings.show_window_info = is_checked(hwnd, IDC_SHOW_INFO);
    settings.use_system_backdrop = is_checked(hwnd, IDC_USE_SYSTEM_BACKDROP);

    settings.refresh_interval_ms = if is_checked(hwnd, IDC_REFRESH_1S) {
        1_000
    } else if is_checked(hwnd, IDC_REFRESH_5S) {
        5_000
    } else if is_checked(hwnd, IDC_REFRESH_10S) {
        10_000
    } else {
        2_000
    };

    settings.initial_layout = selected_layout(hwnd);
    settings.dock_edge = selected_dock_edge(hwnd);
    settings.fixed_width = read_edit_u32(hwnd, IDC_FIXED_WIDTH);
    settings.fixed_height = read_edit_u32(hwnd, IDC_FIXED_HEIGHT);
    settings.background_color_hex = selected_background_color(hwnd).to_owned();
    settings
}

fn selected_layout(hwnd: HWND) -> LayoutType {
    if is_checked(hwnd, IDC_LAYOUT_MOSAIC) {
        LayoutType::Mosaic
    } else if is_checked(hwnd, IDC_LAYOUT_BENTO) {
        LayoutType::Bento
    } else if is_checked(hwnd, IDC_LAYOUT_FIBONACCI) {
        LayoutType::Fibonacci
    } else if is_checked(hwnd, IDC_LAYOUT_COLUMNS) {
        LayoutType::Columns
    } else if is_checked(hwnd, IDC_LAYOUT_ROW) {
        LayoutType::Row
    } else if is_checked(hwnd, IDC_LAYOUT_COLUMN) {
        LayoutType::Column
    } else {
        LayoutType::Grid
    }
}

fn selected_dock_edge(hwnd: HWND) -> Option<DockEdge> {
    if is_checked(hwnd, IDC_DOCK_LEFT) {
        Some(DockEdge::Left)
    } else if is_checked(hwnd, IDC_DOCK_RIGHT) {
        Some(DockEdge::Right)
    } else if is_checked(hwnd, IDC_DOCK_TOP) {
        Some(DockEdge::Top)
    } else if is_checked(hwnd, IDC_DOCK_BOTTOM) {
        Some(DockEdge::Bottom)
    } else {
        None
    }
}

fn selected_background_color(hwnd: HWND) -> &'static str {
    for (index, (_, color_hex)) in BG_PRESETS.iter().enumerate() {
        if is_checked(hwnd, IDC_BG_PRESET_BASE + index as i32) {
            return color_hex;
        }
    }

    BG_PRESETS[0].1
}

fn selected_tag_color(hwnd: HWND) -> &'static str {
    for (index, (_, color_hex)) in TAG_PRESETS.iter().enumerate() {
        if is_checked(hwnd, IDC_TAG_PRESET_BASE + index as i32) {
            return color_hex;
        }
    }

    TAG_PRESETS[0].1
}

fn read_edit_u32(hwnd: HWND, control_id: i32) -> Option<u32> {
    let text = unsafe { read_window_text(GetDlgItem(hwnd, control_id).unwrap_or_default()) };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok().filter(|value| *value > 0)
    }
}

unsafe fn read_window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }

    let mut buffer = vec![0u16; len as usize + 1];
    let copied = GetWindowTextW(hwnd, &mut buffer);
    String::from_utf16_lossy(&buffer[..copied as usize])
}

fn is_checked(hwnd: HWND, control_id: i32) -> bool {
    let control = unsafe { GetDlgItem(hwnd, control_id).unwrap_or_default() };
    unsafe { SendMessageW(control, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 }
}

unsafe fn create_checkbox(
    parent: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    checked: bool,
) -> HWND {
    let handle = create_control(
        parent,
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | 0x0000_0003),
        x,
        y,
        width,
        height,
        id,
    );
    let _ = SendMessageW(
        handle,
        BM_SETCHECK,
        WPARAM(usize::from(if checked { 1u16 } else { 0u16 })),
        LPARAM(0),
    );
    handle
}

unsafe fn create_radio(
    parent: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    extra_style_bits: u32,
    checked: bool,
) -> HWND {
    let handle = create_control(
        parent,
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | extra_style_bits | 0x0000_0009),
        x,
        y,
        width,
        height,
        id,
    );
    let _ = SendMessageW(
        handle,
        BM_SETCHECK,
        WPARAM(usize::from(if checked { 1u16 } else { 0u16 })),
        LPARAM(0),
    );
    handle
}

unsafe fn create_button(
    parent: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> HWND {
    create_control(
        parent,
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        x,
        y,
        width,
        height,
        id,
    )
}

unsafe fn create_edit(
    parent: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    numbers_only: bool,
) -> HWND {
    let extra = if numbers_only { 0x0000_2000 } else { 0 };
    create_control(
        parent,
        w!("EDIT"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | 0x0000_0080 | extra),
        x,
        y,
        width,
        height,
        id,
    )
}

unsafe fn create_group_box(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> HWND {
    create_control(
        parent,
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | 0x0000_0007),
        x,
        y,
        width,
        height,
        0,
    )
}

unsafe fn create_label(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    extra_style_bits: u32,
) -> HWND {
    create_control(
        parent,
        w!("STATIC"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | extra_style_bits),
        x,
        y,
        width,
        height,
        0,
    )
}

unsafe fn create_control(
    parent: HWND,
    class_name: PCWSTR,
    text: &str,
    style: WINDOW_STYLE,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: i32,
) -> HWND {
    let wide = encode_wide(text);
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name,
        PCWSTR(wide.as_ptr()),
        style,
        x,
        y,
        width,
        height,
        parent,
        HMENU(id as isize as *mut c_void),
        None,
        None,
    )
    .unwrap_or_default()
}

fn register_options_class() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
        let class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(options_wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(6isize as *mut c_void),
            lpszClassName: OPTIONS_CLASS_NAME,
            ..Default::default()
        };
        let _ = RegisterClassExW(&class);
    });
}

fn register_tag_dialog_class() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
        let class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(tag_dialog_wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(6isize as *mut c_void),
            lpszClassName: TAG_DIALOG_CLASS_NAME,
            ..Default::default()
        };
        let _ = RegisterClassExW(&class);
    });
}

unsafe fn options_state(hwnd: HWND) -> Option<&'static mut OptionsWindowState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OptionsWindowState;
    (!ptr.is_null()).then_some(&mut *ptr)
}

unsafe fn tag_dialog_state(hwnd: HWND) -> Option<&'static mut TagDialogState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TagDialogState;
    (!ptr.is_null()).then_some(&mut *ptr)
}

const fn low_word(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

fn encode_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}