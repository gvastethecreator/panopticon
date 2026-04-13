//! Native Win32 window subclassing and low-level input handling.

use std::rc::Rc;
use std::time::{Duration, Instant};

use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::input_ops::{decode_mouse_lparam, scroll_pixels_from_wheel_delta};
use slint::ComponentHandle;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::ABN_POSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::dock::{docked_mode_active, is_blocked_dock_syscommand};
use super::dwm::release_thumbnail;
use super::global_hotkey;
use super::tray::{handle_tray_message, TrayAction, WM_TRAYICON};
use crate::{
    queue_action, recompute_and_update_ui, AppState, MainWindow, PendingAction, SavedWndProc,
    TASKBAR_CREATED_MSG,
};

/// Duration after which the scrollbar overlay auto-hides.
pub(crate) const SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(1500);

pub(crate) fn setup_subclass(
    hwnd: HWND,
    state: &Rc<std::cell::RefCell<AppState>>,
    main_window: &MainWindow,
) {
    crate::UI_STATE.with(|slot| *slot.borrow_mut() = Some(state.clone()));
    crate::UI_WINDOW.with(|slot| *slot.borrow_mut() = Some(main_window.as_weak()));

    // SAFETY: `hwnd` is the live main window created by Slint; reading the
    // current WNDPROC via `GetWindowLongPtrW` is a read-only Win32 query.
    let original = unsafe { GetWindowLongPtrW(hwnd, GWL_WNDPROC) };
    crate::ORIGINAL_WNDPROC.with(|slot| slot.set(SavedWndProc::from_raw(original)));

    // SAFETY: `hwnd` is our live window and `subclass_proc` has the correct
    // `WNDPROC` signature (`unsafe extern "system" fn`).  Replacing the
    // procedure is valid for the lifetime of the window; teardown restores
    // the original before the window is destroyed.
    unsafe {
        let subclass_proc_ptr = subclass_proc as *const () as isize;
        let _ = SetWindowLongPtrW(hwnd, GWL_WNDPROC, subclass_proc_ptr);
    }
}

pub(crate) fn teardown_subclass(hwnd: HWND) {
    let original = crate::ORIGINAL_WNDPROC.with(std::cell::Cell::get);
    if !original.is_null() {
        // SAFETY: `original` was captured from the same `hwnd` in
        // `setup_subclass` and has not been freed; restoring it returns
        // the window to its pre-subclass state.
        unsafe {
            let _ = SetWindowLongPtrW(hwnd, GWL_WNDPROC, original.as_raw());
        }
    }
    crate::UI_STATE.with(|slot| *slot.borrow_mut() = None);
    crate::UI_WINDOW.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn hide_scrollbar_if_idle(weak: &slint::Weak<MainWindow>) {
    let should_hide = crate::SCROLL_LAST_ACTIVITY.with(|clock| {
        clock
            .get()
            .is_some_and(|instant| instant.elapsed() >= SCROLLBAR_HIDE_DELAY)
    });
    if should_hide {
        crate::SCROLL_LAST_ACTIVITY.with(|clock| clock.set(None));
        if let Some(window) = weak.upgrade() {
            window.set_scroll_active(false);
        }
    }
}

#[inline]
fn forward_to_original(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let saved = crate::ORIGINAL_WNDPROC.with(std::cell::Cell::get);
    if saved.is_null() {
        // SAFETY: Fallback to the default window procedure when no
        // original WNDPROC was captured; all arguments come from the OS.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: `saved` was obtained from `GetWindowLongPtrW` in `setup_subclass`
    // on this same window; the window is still alive because we are inside its
    // message handler.  `as_wndproc()` performs the `isize → WNDPROC` transmute
    // in a single audited location.
    unsafe { CallWindowProcW(saved.as_wndproc(), hwnd, msg, wparam, lparam) }
}

#[expect(
    clippy::too_many_lines,
    reason = "centralized Win32 message handling keeps the native subclass deterministic"
)]
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(std::sync::atomic::Ordering::Relaxed);
    if taskbar_msg != 0 && msg == taskbar_msg {
        crate::UI_STATE.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                if let Ok(mut guard) = state.try_borrow_mut() {
                    let small = guard.icons.small;
                    if let Some(tray) = guard.tray_icon.as_mut() {
                        tray.readd(small);
                    }
                }
            }
        });
        return forward_to_original(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_TRAYICON => {
            handle_tray_subclass(hwnd, lparam);
            LRESULT(0)
        }
        WM_HOTKEY => {
            if global_hotkey::is_activate_hotkey(wparam.0) {
                queue_action(PendingAction::ActivateMainWindow);
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        WM_SYSKEYDOWN => {
            if wparam.0 as u32 == 0x12 && (lparam.0 & 0x4000_0000) == 0 {
                toggle_toolbar_from_alt_hotkey();
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        crate::WM_APPBAR_CALLBACK => {
            if wparam.0 as u32 == ABN_POSCHANGED {
                queue_action(PendingAction::Reposition);
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            if docked_mode_active() {
                queue_action(PendingAction::Reposition);
            }
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_SYSCOMMAND => {
            if docked_mode_active() && is_blocked_dock_syscommand(wparam.0) {
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        WM_CLOSE => {
            let should_hide = crate::UI_STATE.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .and_then(|state| {
                        state
                            .try_borrow()
                            .ok()
                            .map(|guard| guard.settings.close_to_tray)
                    })
                    .unwrap_or(false)
            });
            if should_hide {
                queue_action(PendingAction::HideToTray);
            } else {
                crate::queue_exit_request();
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if wparam.0 as u32 == 1 {
                let should_hide = crate::UI_STATE.with(|slot| {
                    slot.borrow()
                        .as_ref()
                        .and_then(|state| {
                            state
                                .try_borrow()
                                .ok()
                                .map(|guard| guard.settings.minimize_to_tray)
                        })
                        .unwrap_or(false)
                });
                if should_hide {
                    queue_action(PendingAction::HideToTray);
                }
            }
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_SHOWWINDOW => {
            handle_show_window(wparam);
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_MBUTTONDOWN => {
            let (x, y) = decode_mouse_lparam(lparam.0);
            crate::PAN_STATE.with(|slot| {
                let mut pan = slot.borrow_mut();
                pan.active = true;
                pan.last_x = x;
                pan.last_y = y;
            });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if handle_wheel_scroll(wparam) {
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        WM_MBUTTONUP => {
            crate::PAN_STATE.with(|slot| slot.borrow_mut().active = false);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pan_active = crate::PAN_STATE.with(|slot| slot.borrow().active);
            if pan_active {
                if wparam.0 & 0x0010 == 0 {
                    crate::PAN_STATE.with(|slot| slot.borrow_mut().active = false);
                    return forward_to_original(hwnd, msg, wparam, lparam);
                }
                handle_middle_pan_move(lparam);
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        _ => forward_to_original(hwnd, msg, wparam, lparam),
    }
}

fn handle_show_window(wparam: WPARAM) {
    if wparam.0 != 0 {
        queue_action(PendingAction::Refresh);
    } else {
        crate::UI_STATE.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                if let Ok(mut guard) = state.try_borrow_mut() {
                    for managed_window in &mut guard.windows {
                        release_thumbnail(managed_window);
                    }
                }
            }
        });
    }
}

fn handle_tray_subclass(hwnd: HWND, lparam: LPARAM) {
    let mouse_msg = lparam.0 as u32;
    if mouse_msg == WM_LBUTTONUP {
        queue_action(PendingAction::Tray(TrayAction::Toggle));
    } else if mouse_msg == WM_RBUTTONUP {
        let menu_state = crate::UI_STATE.with(|slot| {
            slot.borrow().as_ref().and_then(|state| {
                state
                    .try_borrow_mut()
                    .ok()
                    .map(|mut guard| super::tray_actions::build_tray_menu_state(&mut guard))
            })
        });
        if let Some(menu_state) = menu_state {
            if let Some(action) = handle_tray_message(hwnd, lparam, &menu_state) {
                queue_action(PendingAction::Tray(action));
            }
        }
    }
}

fn handle_wheel_scroll(wparam: WPARAM) -> bool {
    let delta = (wparam.0 >> 16) as i16;
    let scroll_px = scroll_pixels_from_wheel_delta(delta);
    crate::UI_WINDOW.with(|slot| {
        let Some(window) = slot.borrow().as_ref().and_then(slint::Weak::upgrade) else {
            return false;
        };
        let scroll_h = window.get_scroll_horizontal();
        let scroll_v = window.get_scroll_vertical();
        if scroll_h {
            let scale = window.window().scale_factor();
            let phys = window.window().size();
            let content_width = window.get_content_width();
            let visible = phys.width as f32 / scale;
            let max_scroll = (content_width - visible).max(0.0);
            let new_vx = (window.get_viewport_x() + scroll_px).clamp(-max_scroll, 0.0);
            window.set_viewport_x(new_vx);
            flash_scrollbar(&window);
            true
        } else if scroll_v {
            let scale = window.window().scale_factor();
            let phys = window.window().size();
            let toolbar_h = if window.get_show_toolbar() {
                TOOLBAR_HEIGHT as f32
            } else {
                0.0
            };
            let content_height = window.get_content_height();
            let visible = phys.height as f32 / scale - toolbar_h;
            let max_scroll = (content_height - visible).max(0.0);
            let new_vy = (window.get_viewport_y() + scroll_px).clamp(-max_scroll, 0.0);
            window.set_viewport_y(new_vy);
            flash_scrollbar(&window);
            true
        } else {
            false
        }
    })
}

fn handle_middle_pan_move(lparam: LPARAM) {
    let (x, y) = decode_mouse_lparam(lparam.0);
    crate::PAN_STATE.with(|slot| {
        let mut pan = slot.borrow_mut();
        let dx = x - pan.last_x;
        let dy = y - pan.last_y;
        pan.last_x = x;
        pan.last_y = y;
        crate::UI_WINDOW.with(|window_slot| {
            if let Some(window) = window_slot.borrow().as_ref().and_then(slint::Weak::upgrade) {
                let scale = window.window().scale_factor();
                let scroll_h = window.get_scroll_horizontal();
                let scroll_v = window.get_scroll_vertical();
                let mut scrolled = false;
                if scroll_h {
                    let phys = window.window().size();
                    let content_width = window.get_content_width();
                    let visible = phys.width as f32 / scale;
                    let max_scroll = (content_width - visible).max(0.0);
                    let new_vx =
                        (window.get_viewport_x() + dx as f32 / scale).clamp(-max_scroll, 0.0);
                    window.set_viewport_x(new_vx);
                    scrolled = true;
                }
                if scroll_v {
                    let phys = window.window().size();
                    let toolbar_h = if window.get_show_toolbar() {
                        TOOLBAR_HEIGHT as f32
                    } else {
                        0.0
                    };
                    let content_height = window.get_content_height();
                    let visible = phys.height as f32 / scale - toolbar_h;
                    let max_scroll = (content_height - visible).max(0.0);
                    let new_vy =
                        (window.get_viewport_y() + dy as f32 / scale).clamp(-max_scroll, 0.0);
                    window.set_viewport_y(new_vy);
                    scrolled = true;
                }
                if scrolled {
                    flash_scrollbar(&window);
                }
            }
        });
    });
}

fn flash_scrollbar(window: &MainWindow) {
    window.set_scroll_active(true);
    crate::SCROLL_LAST_ACTIVITY.with(|clock| clock.set(Some(Instant::now())));
}

fn toggle_toolbar_from_alt_hotkey() {
    crate::UI_STATE.with(|state_slot| {
        crate::UI_WINDOW.with(|window_slot| {
            let Some(state) = state_slot.borrow().as_ref().cloned() else {
                return;
            };
            let Some(window) = window_slot.borrow().as_ref().and_then(slint::Weak::upgrade) else {
                return;
            };
            {
                let mut guard = state.borrow_mut();
                if !guard.settings.shortcuts.alt_toggles_toolbar {
                    return;
                }
                guard.settings.show_toolbar = !guard.settings.show_toolbar;
                let _ = guard.settings.save(guard.profile_name.as_deref());
            }
            recompute_and_update_ui(&state, &window);
        });
    });
}
