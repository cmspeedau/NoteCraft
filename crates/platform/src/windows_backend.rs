//! Minimal Win32 window + message loop.
//!
//! M1 scope is deliberately small: register a class, create one overlapped
//! window, show it, and pump messages until `WM_DESTROY` posts a quit. The
//! real chrome (edit control, toolbar, tabs, status bar) arrives in M2/M3.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostQuitMessage,
    RegisterClassW, SW_SHOW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_DESTROY, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

use crate::{PlatformError, RunOutcome, StartupOptions};

const CLASS_NAME: &str = "NoteCraftMainWindow";

/// Convert a Rust string into a NUL-terminated UTF-16 buffer for the wide APIs.
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Win32 error code from the thread's last-error value, as a plain integer.
unsafe fn last_error() -> u32 {
    unsafe { windows::Win32::Foundation::GetLastError().0 }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    unsafe { run_inner(options) }
}

unsafe fn run_inner(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    let instance = unsafe { GetModuleHandleW(None) }
        .map_err(|_| PlatformError::ModuleHandle(unsafe { last_error() }))?;

    let class_name = wide(CLASS_NAME);
    let class_name = windows::core::PCWSTR::from_raw(class_name.as_ptr());

    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };

    // RegisterClassW returns 0 when it fails; ERROR_CLASS_ALREADY_EXISTS is
    // tolerable (a second instance of the process), any other failure is not.
    if unsafe { RegisterClassW(&wc) } == 0 {
        let err = unsafe { last_error() };
        const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;
        if err != ERROR_CLASS_ALREADY_EXISTS {
            return Err(PlatformError::ClassRegistration(err));
        }
    }

    let title = wide(&options.title);
    let title = windows::core::PCWSTR::from_raw(title.as_ptr());

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            title,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
            windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
            windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
            windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|_| PlatformError::WindowCreation(unsafe { last_error() }))?;

    tracing::info!(
        target: "nc_platform",
        title = %options.title,
        files = ?options.open_paths,
        "main window created"
    );

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
    }

    let mut msg = MSG::default();
    // GetMessageW returns >0 for a message, 0 for WM_QUIT, and -1 on error.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(RunOutcome {
        exit_code: msg.wParam.0 as i32,
    })
}
