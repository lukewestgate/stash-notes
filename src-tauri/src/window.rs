//! Showing, hiding and focusing the single main window. The window is created
//! once at startup and never destroyed, which is what makes summoning instant.

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub const MAIN: &str = "main";

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

pub fn state_flags() -> StateFlags {
    StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED
}

/// Show, unminimize and focus, then tell the UI to put the cursor in the editor.
pub fn show(app: &AppHandle) {
    let Some(w) = main_window(app) else { return };
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
    #[cfg(windows)]
    force_foreground(&w);
    let _ = app.emit("summoned", ());
}

/// Hide to the tray. Windows hands focus back to the previously active window.
pub fn hide(app: &AppHandle) {
    let Some(w) = main_window(app) else { return };
    crate::buffers::flush_all(app);
    let _ = app.save_window_state(state_flags());
    let _ = w.hide();
}

/// The hotkey/tray toggle: hidden -> show; visible and focused -> hide; visible but unfocused -> focus.
pub fn toggle(app: &AppHandle) {
    let Some(w) = main_window(app) else { return };
    let visible = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
    if visible && w.is_focused().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}

/// Windows' foreground lock can refuse focus to a window shown by a background
/// process. If `set_focus` did not take, briefly attach to the foreground
/// thread's input queue, which lets `SetForegroundWindow` succeed.
#[cfg(windows)]
fn force_foreground(w: &WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };
    let Ok(hwnd) = w.hwnd() else { return };
    let hwnd = hwnd.0 as HWND;
    unsafe {
        let fg = GetForegroundWindow();
        if fg == hwnd {
            return;
        }
        let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, 1) != 0;
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if attached {
            AttachThreadInput(me, fg_thread, 0);
        }
    }
    let _ = w.set_focus();
}
