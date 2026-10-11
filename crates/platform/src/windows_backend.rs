//! Minimal Win32 window + message loop with a registry-driven menu bar.
//!
//! The menu bar is not hand-written here: it is built from `nc-commands`, the
//! single authoritative specification of the application's commands. This
//! module only translates each resolved menu item into a Win32 `HMENU` and
//! routes `WM_COMMAND` back through the same registry, so the command surface
//! has exactly one definition.
//!
//! The client area (edit control, toolbar, tabs, status bar) arrives in M2/M3;
//! creating the window is exercised by the Windows CI job.

use nc_commands::{AvailabilityContext, MenuItem, MenuModel};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW, DrawMenuBar,
    GetMessageW, HMENU, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, PostQuitMessage,
    RegisterClassW, SW_SHOW, SetMenu, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_COMMAND,
    WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
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

struct ShellState {
    /// The registry context the menu is currently rendered for.
    context: AvailabilityContext,
}

impl ShellState {
    /// The state at first launch: one empty, clean, unselected document.
    fn new() -> Self {
        Self {
            context: AvailabilityContext::initial(),
        }
    }
}

/// Routes a `WM_COMMAND` menu id through the registry.
///
/// The registry says whether the command is available and which milestone
/// first gives it a real effect; the *effect* itself belongs to the handler
/// introduced with that milestone, so here the route is recorded and nothing
/// else happens.
fn route_command(state: &ShellState, menu_id: u16) {
    match nc_commands::by_menu_id(menu_id) {
        Some(spec) => tracing::info!(
            target: "nc_platform",
            command = spec.id.as_str(),
            label = spec.label,
            milestone = %spec.milestone,
            enabled = spec.is_available(state.context),
            "menu command routed"
        ),
        None => tracing::warn!(target: "nc_platform", menu_id, "unknown menu command id"),
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND => {
                // The low word of w_param carries the command id. The state is
                // still the launch state until M2's shell state lands.
                let menu_id = (wparam.0 & 0xFFFF) as u16;
                route_command(&ShellState::new(), menu_id);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Builds a menu bar from the resolved registry model.
unsafe fn build_menu_bar(context: AvailabilityContext) -> Result<HMENU, PlatformError> {
    unsafe {
        let bar = CreateMenu().map_err(|_| PlatformError::MenuCreation(last_error()))?;
        for model in nc_commands::build_menu_bar(context) {
            let popup = append_popup(bar, &model)?;
            debug_assert!(!popup.0.is_null());
        }
        Ok(bar)
    }
}

/// Appends one top-level menu, returning its popup handle.
unsafe fn append_popup(parent: HMENU, model: &MenuModel) -> Result<HMENU, PlatformError> {
    unsafe {
        let popup = CreateMenu().map_err(|_| PlatformError::MenuCreation(last_error()))?;
        append_items(popup, &model.items)?;
        let label = wide(&format!("&{}", model.menu.label()));
        AppendMenuW(
            parent,
            MF_POPUP | MF_STRING,
            popup.0 as usize,
            windows::core::PCWSTR::from_raw(label.as_ptr()),
        )
        .map_err(|_| PlatformError::MenuCreation(last_error()))?;
        Ok(popup)
    }
}

/// Appends a resolved item list to `menu`, recursively for submenus.
unsafe fn append_items(menu: HMENU, items: &[MenuItem]) -> Result<(), PlatformError> {
    unsafe {
        for item in items {
            match item {
                MenuItem::Separator => {
                    AppendMenuW(menu, MF_SEPARATOR, 0, windows::core::PCWSTR::null())
                        .map_err(|_| PlatformError::MenuCreation(last_error()))?;
                }
                MenuItem::Command { spec, enabled, .. } => {
                    let label = wide(spec.label);
                    let flags = if *enabled {
                        MF_STRING
                    } else {
                        MF_STRING | MF_GRAYED
                    };
                    AppendMenuW(
                        menu,
                        flags,
                        spec.menu_id as usize,
                        windows::core::PCWSTR::from_raw(label.as_ptr()),
                    )
                    .map_err(|_| PlatformError::MenuCreation(last_error()))?;
                }
                MenuItem::Submenu { title, items } => {
                    let submenu =
                        CreateMenu().map_err(|_| PlatformError::MenuCreation(last_error()))?;
                    append_items(submenu, items)?;
                    let label = wide(title);
                    AppendMenuW(
                        menu,
                        MF_POPUP | MF_STRING,
                        submenu.0 as usize,
                        windows::core::PCWSTR::from_raw(label.as_ptr()),
                    )
                    .map_err(|_| PlatformError::MenuCreation(last_error()))?;
                }
            }
        }
        Ok(())
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

    let state = ShellState::new();
    let menu = unsafe { build_menu_bar(state.context)? };

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
            Some(menu),
            Some(instance.into()),
            None,
        )
    }
    .map_err(|_| PlatformError::WindowCreation(unsafe { last_error() }))?;

    tracing::info!(
        target: "nc_platform",
        title = %options.title,
        files = ?options.open_paths,
        commands = nc_commands::command_count(),
        "main window created"
    );

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
        let _ = SetMenu(hwnd, Some(menu));
        let _ = DrawMenuBar(hwnd);
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
