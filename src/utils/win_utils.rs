use crate::APP_TITLE;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use winapi::shared::minwindef::{BOOL, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    EnumWindows, GetWindowLongA, GetWindowTextA, SetWindowLongA, SetWindowPos, GWL_STYLE,
    SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_MAXIMIZEBOX,
};

/// Disables the maximize button for all windows containing APP_TITLE in their title.
/// Returns Some message on success, None if no matching window was found.
pub fn disable_maximize_button_for_all() -> Option<&'static str> {
    let found = Arc::new(AtomicBool::new(false));
    let found_clone = found.clone();

    unsafe {
        extern "system" fn enum_callback(hwnd: HWND, found: LPARAM) -> BOOL {
            let window_title = unsafe { get_window_title(hwnd) };

            if let Some(window_title) = window_title {
                if window_title.contains(APP_TITLE) {
                    #[cfg(debug_assertions)]
                    println!("Found window: {}", window_title);

                    unsafe { disable_maximize_for_window(hwnd) };
                    unsafe { mark_window_found(found) };
                }
            }
            TRUE
        }

        EnumWindows(Some(enum_callback), &*found_clone as *const _ as LPARAM);
    }

    if found.load(Ordering::SeqCst) {
        Some("Window controls set successfully")
    } else {
        None
    }
}

/// Retrieves the window title from a window handle.
unsafe fn get_window_title(hwnd: HWND) -> Option<String> { unsafe {
    // Buffer for the window title (512 bytes should be enough for most window titles)
    let mut title = vec![0u8; 512];

    let len = GetWindowTextA(hwnd, title.as_mut_ptr() as *mut i8, title.len() as i32);
    if len == 0 {
        return None;
    }

    // Truncate to the actual length returned
    title.truncate(len as usize);

    // Convert to String, handling any invalid UTF-8
    std::ffi::CString::new(title)
        .ok().map(|c_string| c_string.to_string_lossy().into_owned())
}}

/// Disables the maximize button for a specific window.
unsafe fn disable_maximize_for_window(hwnd: HWND) { unsafe {
    let style = GetWindowLongA(hwnd, GWL_STYLE) as u32;
    let new_style = style & !WS_MAXIMIZEBOX;
    SetWindowLongA(hwnd, GWL_STYLE, new_style as i32);

    // Force a redraw of the window frame
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
    );
}}

/// Marks the window as found using the atomic flag.
unsafe fn mark_window_found(found: LPARAM) { unsafe {
    let found_ptr = found as *mut AtomicBool;
    (*found_ptr).store(true, Ordering::SeqCst);
}}

/// Sets up window controls in a background thread with retry logic.
pub fn setup_window_controls() {
    const MAX_ATTEMPTS: u64 = 20;
    const INITIAL_DELAY_MS: u64 = 50;
    const RETRY_DELAY_MS: u64 = 50;

    std::thread::spawn(|| {
        // Try immediately first
        if disable_maximize_button_for_all().is_some() {
            #[cfg(debug_assertions)]
            println!("Window controls set successfully");
            return;
        }

        // Very short initial delay before rapid attempts
        std::thread::sleep(Duration::from_millis(INITIAL_DELAY_MS));

        // Rapid attempts at first
        for _ in 0..5 {
            if disable_maximize_button_for_all().is_some() {
                #[cfg(debug_assertions)]
                println!("Window controls set successfully");
                return;
            }
            std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
        }

        // If still not found, try with increasing delays
        for i in 1..=MAX_ATTEMPTS {
            if disable_maximize_button_for_all().is_some() {
                #[cfg(debug_assertions)]
                println!("Window controls set successfully");
                break;
            }
            std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS * i));
        }
    });
}
