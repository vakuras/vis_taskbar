use crate::config::Settings;
use crate::tray::{self, show_color_dialog};

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use std::sync::{Arc, Mutex};

// Control IDs
const IDC_FULL_TASKBAR: u32 = 1001;
const IDC_BARS: u32 = 1002;
const IDC_SLEEP_LABEL: u32 = 1003;
const IDC_SLEEP_EDIT: u32 = 1004;
const IDC_STEP_LABEL: u32 = 1005;
const IDC_STEP_EDIT: u32 = 1006;
const IDC_TOP_COLOR_BTN: u32 = 1010;
const IDC_BOTTOM_COLOR_BTN: u32 = 1011;
const IDC_PEAK_COLOR_BTN: u32 = 1012;
const IDC_TOP_COLOR_SWATCH: u32 = 1020;
const IDC_BOTTOM_COLOR_SWATCH: u32 = 1021;
const IDC_PEAK_COLOR_SWATCH: u32 = 1022;
const IDC_APPLY: u32 = 1030;
const IDC_RESET: u32 = 1031;
const IDC_CLOSE: u32 = 1032;

const PREFS_CLASS: PCWSTR = w!("VIS_PREFS_CLASS");

struct UiState {
    settings: Arc<Mutex<Settings>>,
    local: Settings,
    tray: Option<tray::TrayIcon>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_fn: Box<dyn Fn()>,
    stop_fn: Box<dyn Fn()>,
}

/// Create and run the preferences window + message loop.
pub fn run_ui(
    settings: Arc<Mutex<Settings>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_fn: Box<dyn Fn()>,
    stop_fn: Box<dyn Fn()>,
) {
    unsafe {
        let hinstance: HINSTANCE = std::mem::transmute(GetModuleHandleW(None).unwrap());

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(prefs_wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as *mut _),
            lpszClassName: PREFS_CLASS,
            ..std::mem::zeroed()
        };

        RegisterClassExW(&wc);

        let local = settings.lock().unwrap().clone();
        let state = Box::new(UiState {
            settings,
            local,
            tray: None,
            is_running,
            start_fn,
            stop_fn,
        });
        let state_ptr = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PREFS_CLASS,
            w!("vis_taskbar - Preferences"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            320,
            380,
            HWND::default(),
            HMENU::default(),
            hinstance,
            Some(state_ptr as *const _),
        )
        .unwrap();

        // Create tray icon
        if let Ok(tray) = tray::TrayIcon::new(hwnd) {
            (*state_ptr).tray = Some(tray);
        }

        create_controls(hwnd, hinstance, &(*state_ptr).local);

        // Start hidden
        ShowWindow(hwnd, SW_HIDE);

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = Box::from_raw(state_ptr);
    }
}

unsafe fn create_controls(hwnd: HWND, hinstance: HINSTANCE, settings: &Settings) {
    let font = GetStockObject(DEFAULT_GUI_FONT);

    let create = |class: PCWSTR, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: u32| {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let child = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(wide.as_ptr()),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | style),
            x, y, w, h,
            hwnd,
            HMENU(id as *mut _),
            hinstance,
            None,
        ).unwrap();
        SendMessageW(child, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
        child
    };

    // Checkboxes
    create(w!("BUTTON"), "Full Taskbar", BS_AUTOCHECKBOX as u32, 20, 15, 140, 20, IDC_FULL_TASKBAR);
    create(w!("BUTTON"), "Bar Mode", BS_AUTOCHECKBOX as u32, 170, 15, 120, 20, IDC_BARS);

    if settings.full_taskbar {
        SendDlgItemMessageW(hwnd, IDC_FULL_TASKBAR as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));
    }
    if settings.bars {
        SendDlgItemMessageW(hwnd, IDC_BARS as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));
    }

    // Sleep time
    create(w!("STATIC"), "Sleep (ms):", 0, 20, 50, 80, 20, IDC_SLEEP_LABEL);
    create(w!("EDIT"), &settings.sleep_time_ms.to_string(), WS_BORDER.0 | ES_NUMBER as u32, 110, 48, 60, 22, IDC_SLEEP_EDIT);

    // Step multiplier
    create(w!("STATIC"), "Step:", 0, 20, 80, 80, 20, IDC_STEP_LABEL);
    create(w!("EDIT"), &settings.step_multiplier.to_string(), WS_BORDER.0 | ES_NUMBER as u32, 110, 78, 60, 22, IDC_STEP_EDIT);

    // Color buttons + swatches
    let color_y = 120;
    create(w!("STATIC"), "", 0, 20, color_y, 30, 20, IDC_TOP_COLOR_SWATCH);
    create(w!("BUTTON"), "Top Color...", 0, 60, color_y, 110, 24, IDC_TOP_COLOR_BTN);

    create(w!("STATIC"), "", 0, 20, color_y + 30, 30, 20, IDC_BOTTOM_COLOR_SWATCH);
    create(w!("BUTTON"), "Bottom Color...", 0, 60, color_y + 30, 110, 24, IDC_BOTTOM_COLOR_BTN);

    create(w!("STATIC"), "", 0, 20, color_y + 60, 30, 20, IDC_PEAK_COLOR_SWATCH);
    create(w!("BUTTON"), "Peak Color...", 0, 60, color_y + 60, 110, 24, IDC_PEAK_COLOR_BTN);

    // Buttons
    let btn_y = 280;
    create(w!("BUTTON"), "Apply", 0, 20, btn_y, 80, 28, IDC_APPLY);
    create(w!("BUTTON"), "Reset", 0, 110, btn_y, 80, 28, IDC_RESET);
    create(w!("BUTTON"), "Close", 0, 200, btn_y, 80, 28, IDC_CLOSE);
}

fn get_edit_u32(hwnd: HWND, id: u32) -> u32 {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id as i32).unwrap_or_default();
        let mut buf = [0u16; 16];
        let len = GetWindowTextW(ctrl, &mut buf);
        let s = String::from_utf16_lossy(&buf[..len as usize]);
        s.parse().unwrap_or(1)
    }
}

fn is_checked(hwnd: HWND, id: u32) -> bool {
    unsafe {
        let result = SendDlgItemMessageW(hwnd, id as i32, BM_GETCHECK, WPARAM(0), LPARAM(0));
        result.0 == 1 // BST_CHECKED
    }
}

unsafe extern "system" fn prefs_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if state_ptr == 0 {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let state = &mut *(state_ptr as *mut UiState);
            let ctrl_hwnd = HWND(lparam.0 as *mut _);
            let id = GetDlgCtrlID(ctrl_hwnd) as u32;
            let hdc = HDC(wparam.0 as *mut _);

            let color = match id {
                IDC_TOP_COLOR_SWATCH => Some(state.local.color_top),
                IDC_BOTTOM_COLOR_SWATCH => Some(state.local.color_bottom),
                IDC_PEAK_COLOR_SWATCH => Some(state.local.color_peaks),
                _ => None,
            };

            if let Some(c) = color {
                let cr = COLORREF(c.to_colorref());
                SetBkColor(hdc, cr);
                let brush = CreateSolidBrush(cr);
                return LRESULT(brush.0 as isize);
            }

            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_COMMAND => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if state_ptr == 0 {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let state = &mut *(state_ptr as *mut UiState);
            let cmd = (wparam.0 & 0xFFFF) as u32;

            match cmd {
                IDC_FULL_TASKBAR => {
                    state.local.full_taskbar = is_checked(hwnd, IDC_FULL_TASKBAR);
                }
                IDC_BARS => {
                    state.local.bars = is_checked(hwnd, IDC_BARS);
                }
                IDC_TOP_COLOR_BTN => {
                    state.local.color_top = show_color_dialog(hwnd, state.local.color_top);
                    let _ = InvalidateRect(hwnd, None, true);
                }
                IDC_BOTTOM_COLOR_BTN => {
                    state.local.color_bottom = show_color_dialog(hwnd, state.local.color_bottom);
                    let _ = InvalidateRect(hwnd, None, true);
                }
                IDC_PEAK_COLOR_BTN => {
                    state.local.color_peaks = show_color_dialog(hwnd, state.local.color_peaks);
                    let _ = InvalidateRect(hwnd, None, true);
                }
                IDC_APPLY => {
                    state.local.sleep_time_ms = get_edit_u32(hwnd, IDC_SLEEP_EDIT);
                    state.local.step_multiplier = get_edit_u32(hwnd, IDC_STEP_EDIT);
                    *state.settings.lock().unwrap() = state.local.clone();
                    if let Err(e) = state.local.save() {
                        log::error!("Failed to save config: {e}");
                    }
                }
                IDC_RESET => {
                    state.local = state.settings.lock().unwrap().clone();
                    let _ = InvalidateRect(hwnd, None, true);
                }
                IDC_CLOSE => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                cmd if cmd == tray::CMD_SHOW_CONFIG => {
                    ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                cmd if cmd == tray::CMD_HIDE_CONFIG => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                cmd if cmd == tray::CMD_START => {
                    (state.start_fn)();
                }
                cmd if cmd == tray::CMD_STOP => {
                    (state.stop_fn)();
                }
                cmd if cmd == tray::CMD_EXIT => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        msg if msg == tray::TRAY_MSG => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if state_ptr == 0 {
                return LRESULT(0);
            }
            let state = &*(state_ptr as *const UiState);
            let event = (lparam.0 & 0xFFFF) as u32;
            match event {
                WM_LBUTTONDBLCLK => {
                    ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                WM_RBUTTONDOWN | WM_CONTEXTMENU => {
                    if let Some(tray) = &state.tray {
                        let running = state.is_running.load(std::sync::atomic::Ordering::Relaxed);
                        tray.show_context_menu(running);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
