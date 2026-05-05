//! Pure routing for Win32 messages intercepted by Window Subclassing.

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F1;
use windows::Win32::UI::Shell::ABN_POSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_CLOSE, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_HOTKEY, WM_KEYDOWN, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_SETTINGCHANGE, WM_SHOWWINDOW, WM_SIZE, WM_SYSCOMMAND,
    WM_SYSKEYDOWN, WM_WINDOWPOSCHANGED,
};

use super::dock::is_blocked_dock_syscommand;
use super::global_hotkey;
use super::tray::WM_TRAYICON;

const ALT_VIRTUAL_KEY: u32 = 0x12;
const KEY_REPEAT_LPARAM_MASK: isize = 0x4000_0000;
const SIZE_MINIMIZED: usize = 1;
const MOUSE_MIDDLE_BUTTON_WPARAM: usize = 0x0010;

#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeMessage {
    pub(crate) hwnd: HWND,
    pub(crate) msg: u32,
    pub(crate) wparam: WPARAM,
    pub(crate) lparam: LPARAM,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDispatch {
    Handled,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeEvent {
    TaskbarCreated,
    TrayIcon { mouse_msg: u32 },
    ActivateHotkey,
    OpenAboutHotkey,
    ToggleToolbarHotkey,
    AppbarPositionChanged,
    DockSurfaceChanged,
    BlockDockSysCommand,
    CloseRequested,
    MinimizeRequested,
    WindowShown,
    WindowHidden,
    MiddlePanStart,
    MiddlePanEnd,
    MiddlePanMove { middle_button_down: bool },
    Wheel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeRoute {
    pub(crate) event: Option<NativeEvent>,
    pub(crate) dispatch: NativeDispatch,
}

impl NativeRoute {
    const fn handled(event: NativeEvent) -> Self {
        Self {
            event: Some(event),
            dispatch: NativeDispatch::Handled,
        }
    }

    const fn forward(event: Option<NativeEvent>) -> Self {
        Self {
            event,
            dispatch: NativeDispatch::Forward,
        }
    }
}

pub(crate) fn route_native_message(
    message: NativeMessage,
    taskbar_created_msg: u32,
    appbar_callback_msg: u32,
    docked: bool,
    pan_active: bool,
) -> NativeRoute {
    let _ = message.hwnd;
    if taskbar_created_msg != 0 && message.msg == taskbar_created_msg {
        return NativeRoute::forward(Some(NativeEvent::TaskbarCreated));
    }

    match message.msg {
        WM_TRAYICON => NativeRoute::handled(NativeEvent::TrayIcon {
            mouse_msg: message.lparam.0 as u32,
        }),
        WM_HOTKEY if global_hotkey::is_activate_hotkey(message.wparam.0) => {
            NativeRoute::handled(NativeEvent::ActivateHotkey)
        }
        WM_KEYDOWN
            if message.wparam.0 as u32 == u32::from(VK_F1.0)
                && !is_repeated_key(message.lparam) =>
        {
            NativeRoute::handled(NativeEvent::OpenAboutHotkey)
        }
        WM_SYSKEYDOWN
            if message.wparam.0 as u32 == ALT_VIRTUAL_KEY && !is_repeated_key(message.lparam) =>
        {
            NativeRoute::handled(NativeEvent::ToggleToolbarHotkey)
        }
        msg if msg == appbar_callback_msg => {
            let event = (message.wparam.0 as u32 == ABN_POSCHANGED)
                .then_some(NativeEvent::AppbarPositionChanged);
            NativeRoute {
                event,
                dispatch: NativeDispatch::Handled,
            }
        }
        WM_WINDOWPOSCHANGED | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            NativeRoute::forward(docked.then_some(NativeEvent::DockSurfaceChanged))
        }
        WM_SYSCOMMAND if docked && is_blocked_dock_syscommand(message.wparam.0) => {
            NativeRoute::handled(NativeEvent::BlockDockSysCommand)
        }
        WM_CLOSE => NativeRoute::handled(NativeEvent::CloseRequested),
        WM_SIZE if message.wparam.0 == SIZE_MINIMIZED => {
            NativeRoute::forward(Some(NativeEvent::MinimizeRequested))
        }
        WM_SHOWWINDOW if message.wparam.0 != 0 => {
            NativeRoute::forward(Some(NativeEvent::WindowShown))
        }
        WM_SHOWWINDOW => NativeRoute::forward(Some(NativeEvent::WindowHidden)),
        WM_MBUTTONDOWN => NativeRoute::handled(NativeEvent::MiddlePanStart),
        WM_MOUSEWHEEL => NativeRoute::handled(NativeEvent::Wheel),
        WM_MBUTTONUP => NativeRoute::handled(NativeEvent::MiddlePanEnd),
        WM_MOUSEMOVE if pan_active => NativeRoute::handled(NativeEvent::MiddlePanMove {
            middle_button_down: message.wparam.0 & MOUSE_MIDDLE_BUTTON_WPARAM != 0,
        }),
        _ => NativeRoute::forward(None),
    }
}

const fn is_repeated_key(lparam: LPARAM) -> bool {
    lparam.0 & KEY_REPEAT_LPARAM_MASK != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use windows::Win32::UI::WindowsAndMessaging::{
        SC_MOVE, WM_DISPLAYCHANGE, WM_LBUTTONUP, WM_RBUTTONUP,
    };

    const TASKBAR_CREATED: u32 = 0x8001;
    const APPBAR_CALLBACK: u32 = 0x8002;

    fn msg(msg: u32, wparam: usize, lparam: isize) -> NativeMessage {
        NativeMessage {
            hwnd: HWND(1usize as *mut c_void),
            msg,
            wparam: WPARAM(wparam),
            lparam: LPARAM(lparam),
        }
    }

    #[test]
    fn routes_hotkeys_without_repeats() {
        assert_eq!(
            route_native_message(
                msg(
                    WM_HOTKEY,
                    global_hotkey::GLOBAL_ACTIVATE_HOTKEY_ID as usize,
                    0
                ),
                TASKBAR_CREATED,
                APPBAR_CALLBACK,
                false,
                false,
            ),
            NativeRoute::handled(NativeEvent::ActivateHotkey)
        );
        assert_eq!(
            route_native_message(
                msg(WM_KEYDOWN, VK_F1.0 as usize, 0),
                0,
                APPBAR_CALLBACK,
                false,
                false
            ),
            NativeRoute::handled(NativeEvent::OpenAboutHotkey)
        );
        assert_eq!(
            route_native_message(
                msg(WM_KEYDOWN, VK_F1.0 as usize, KEY_REPEAT_LPARAM_MASK),
                0,
                APPBAR_CALLBACK,
                false,
                false,
            ),
            NativeRoute::forward(None)
        );
    }

    #[test]
    fn routes_tray_and_appbar_messages() {
        assert_eq!(
            route_native_message(
                msg(WM_TRAYICON, 0, WM_LBUTTONUP as isize),
                0,
                APPBAR_CALLBACK,
                false,
                false
            ),
            NativeRoute::handled(NativeEvent::TrayIcon {
                mouse_msg: WM_LBUTTONUP
            })
        );
        assert_eq!(
            route_native_message(
                msg(WM_TRAYICON, 0, WM_RBUTTONUP as isize),
                0,
                APPBAR_CALLBACK,
                false,
                false
            ),
            NativeRoute::handled(NativeEvent::TrayIcon {
                mouse_msg: WM_RBUTTONUP
            })
        );
        assert_eq!(
            route_native_message(
                msg(APPBAR_CALLBACK, ABN_POSCHANGED as usize, 0),
                0,
                APPBAR_CALLBACK,
                true,
                false,
            ),
            NativeRoute::handled(NativeEvent::AppbarPositionChanged)
        );
        assert_eq!(
            route_native_message(msg(APPBAR_CALLBACK, 99, 0), 0, APPBAR_CALLBACK, true, false),
            NativeRoute {
                event: None,
                dispatch: NativeDispatch::Handled,
            }
        );
        assert_eq!(
            route_native_message(
                msg(TASKBAR_CREATED, 0, 0),
                TASKBAR_CREATED,
                APPBAR_CALLBACK,
                false,
                false,
            ),
            NativeRoute::forward(Some(NativeEvent::TaskbarCreated))
        );
    }

    #[test]
    fn routes_dock_surface_and_blocked_syscommands_conditionally() {
        assert_eq!(
            route_native_message(msg(WM_DISPLAYCHANGE, 0, 0), 0, APPBAR_CALLBACK, true, false),
            NativeRoute::forward(Some(NativeEvent::DockSurfaceChanged))
        );
        assert_eq!(
            route_native_message(
                msg(WM_DISPLAYCHANGE, 0, 0),
                0,
                APPBAR_CALLBACK,
                false,
                false
            ),
            NativeRoute::forward(None)
        );
        assert_eq!(
            route_native_message(
                msg(WM_SYSCOMMAND, SC_MOVE as usize, 0),
                0,
                APPBAR_CALLBACK,
                true,
                false
            ),
            NativeRoute::handled(NativeEvent::BlockDockSysCommand)
        );
    }

    #[test]
    fn routes_window_lifecycle_and_pan_messages() {
        assert_eq!(
            route_native_message(msg(WM_CLOSE, 0, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::handled(NativeEvent::CloseRequested)
        );
        assert_eq!(
            route_native_message(
                msg(WM_SIZE, SIZE_MINIMIZED, 0),
                0,
                APPBAR_CALLBACK,
                false,
                false
            ),
            NativeRoute::forward(Some(NativeEvent::MinimizeRequested))
        );
        assert_eq!(
            route_native_message(msg(WM_SHOWWINDOW, 1, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::forward(Some(NativeEvent::WindowShown))
        );
        assert_eq!(
            route_native_message(msg(WM_MOUSEMOVE, 0, 0), 0, APPBAR_CALLBACK, false, true),
            NativeRoute::handled(NativeEvent::MiddlePanMove {
                middle_button_down: false
            })
        );
        assert_eq!(
            route_native_message(msg(WM_MOUSEMOVE, 0, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::forward(None)
        );
        assert_eq!(
            route_native_message(msg(WM_SHOWWINDOW, 0, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::forward(Some(NativeEvent::WindowHidden))
        );
        assert_eq!(
            route_native_message(msg(WM_MBUTTONDOWN, 0, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::handled(NativeEvent::MiddlePanStart)
        );
        assert_eq!(
            route_native_message(msg(WM_MBUTTONUP, 0, 0), 0, APPBAR_CALLBACK, false, true),
            NativeRoute::handled(NativeEvent::MiddlePanEnd)
        );
        assert_eq!(
            route_native_message(msg(WM_MOUSEWHEEL, 0, 0), 0, APPBAR_CALLBACK, false, false),
            NativeRoute::handled(NativeEvent::Wheel)
        );
    }
}
