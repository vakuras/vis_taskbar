use crate::config::Settings;
use crate::tray::{self, show_color_dialog};

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows::core::*;

use std::sync::{Arc, Mutex};

// Control IDs
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
const IDC_RESTART: u32 = 1033;
// Section labels
const IDC_SECTION_DISPLAY: u32 = 1040;
const IDC_SECTION_TIMING: u32 = 1041;
const IDC_SECTION_COLORS: u32 = 1042;
const IDC_TOP_COLOR_LABEL: u32 = 1043;
const IDC_BOTTOM_COLOR_LABEL: u32 = 1044;
const IDC_PEAK_COLOR_LABEL: u32 = 1045;
const IDC_PREVIEW: u32 = 1050;
// Advanced section
const IDC_SECTION_ADVANCED: u32 = 1060;
const IDC_WINDOW_HANN: u32 = 1061;
const IDC_WINDOW_HAMMING: u32 = 1062;
const IDC_WINDOW_BH: u32 = 1063;
const IDC_CUTOFF_LABEL: u32 = 1064;
const IDC_CUTOFF_EDIT: u32 = 1065;
const IDC_MERGE_MAX: u32 = 1066;
const IDC_MERGE_AVG: u32 = 1067;
const IDC_LOG_SPREAD: u32 = 1068;
const IDC_GAIN_LABEL: u32 = 1069;
const IDC_GAIN_EDIT: u32 = 1070;
const IDC_WINDOW_LABEL: u32 = 1071;
const IDC_MERGE_LABEL: u32 = 1072;

const PREVIEW_TIMER_ID: usize = 100;
const PREVIEW_HEIGHT: i32 = 60;

const PREFS_CLASS: PCWSTR = w!("VIS_PREFS_CLASS");

// Eventide "Midnight" theme colors (COLORREF = 0x00BBGGRR)
const BG_COLOR: u32 = 0x0028201E;       // #1e2028 base background
const SURFACE_COLOR: u32 = 0x00362A28;  // #282a36 surface/card
const OVERLAY_COLOR: u32 = 0x00211816;  // #161821 input/overlay
const TEXT_COLOR: u32 = 0x00EDEAE8;     // #e8eaed primary text
const TEXT_DIM: u32 = 0x00A6A09A;       // #9aa0a6 secondary text
const ACCENT_COLOR: u32 = 0x00F8B48A;   // #8ab4f8 blue accent
const ACCENT_HOVER: u32 = 0x00FACBAE;   // #aecbfa accent hover
const BORDER_COLOR: u32 = 0x004A3B38;   // #383b4a border

static mut BG_BRUSH: HBRUSH = HBRUSH(std::ptr::null_mut());
static mut SURFACE_BRUSH: HBRUSH = HBRUSH(std::ptr::null_mut());
static mut OVERLAY_BRUSH: HBRUSH = HBRUSH(std::ptr::null_mut());
static mut UI_FONT: HFONT = HFONT(std::ptr::null_mut());
static mut SECTION_FONT: HFONT = HFONT(std::ptr::null_mut());

struct UiState {
    settings: Arc<Mutex<Settings>>,
    local: Settings,
    tray: Option<tray::TrayIcon>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_fn: Box<dyn Fn()>,
    stop_fn: Box<dyn Fn()>,
    shared_falloff: Arc<Mutex<crate::renderer::SharedFalloff>>,
    preview_renderer: Option<crate::renderer::Renderer>,
}

/// Create and run the preferences window + message loop.
pub fn run_ui(
    settings: Arc<Mutex<Settings>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_fn: Box<dyn Fn()>,
    stop_fn: Box<dyn Fn()>,
    shared_falloff: Arc<Mutex<crate::renderer::SharedFalloff>>,
) {
    unsafe {
        let hinstance: HINSTANCE = std::mem::transmute(GetModuleHandleW(None).unwrap());

        // Create dark background brush and fonts
        BG_BRUSH = CreateSolidBrush(COLORREF(BG_COLOR));
        SURFACE_BRUSH = CreateSolidBrush(COLORREF(SURFACE_COLOR));
        OVERLAY_BRUSH = CreateSolidBrush(COLORREF(OVERLAY_COLOR));

        let font_name: Vec<u16> = "Segoe UI".encode_utf16().chain(std::iter::once(0)).collect();
        let mut lf: LOGFONTW = std::mem::zeroed();
        lf.lfHeight = -14;
        lf.lfWeight = 400; // FW_NORMAL
        lf.lfQuality = CLEARTYPE_QUALITY;
        lf.lfFaceName[..font_name.len()].copy_from_slice(&font_name);
        UI_FONT = CreateFontIndirectW(&lf);

        lf.lfHeight = -13;
        lf.lfWeight = 600; // FW_SEMIBOLD
        SECTION_FONT = CreateFontIndirectW(&lf);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(prefs_wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap_or_default(),
            hbrBackground: BG_BRUSH,
            lpszClassName: PREFS_CLASS,
            hIcon: LoadIconW(hinstance, PCWSTR(1 as *const u16)).unwrap_or_default(),
            hIconSm: LoadIconW(hinstance, PCWSTR(1 as *const u16)).unwrap_or_default(),
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
            shared_falloff,
            preview_renderer: None,
        });
        let state_ptr = Box::into_raw(state);

        let win_w = 480;
        let win_h = 570;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PREFS_CLASS,
            w!("vis_taskbar v0.7.0 - Settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            win_w,
            win_h,
            HWND::default(),
            HMENU::default(),
            hinstance,
            Some(state_ptr as *const _),
        )
        .unwrap();

        // Enable dark title bar (Windows 10 20H1+ / Windows 11)
        enable_dark_mode(hwnd);

        // Create tray icon
        if let Ok(tray) = tray::TrayIcon::new(hwnd) {
            (*state_ptr).tray = Some(tray);
        }

        create_controls(hwnd, hinstance, &(*state_ptr).local);

        // Start a timer for preview updates (match taskbar refresh)
        SetTimer(hwnd, PREVIEW_TIMER_ID, 15, None);

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
        DeleteObject(BG_BRUSH);
        DeleteObject(SURFACE_BRUSH);
        DeleteObject(OVERLAY_BRUSH);
        DeleteObject(UI_FONT);
        DeleteObject(SECTION_FONT);
    }
}

unsafe fn create_controls(hwnd: HWND, hinstance: HINSTANCE, settings: &Settings) {
    let pad = 20i32;       // outer padding
    let inner_pad = 15i32; // padding inside sections
    let client_w = 420i32; // usable width inside padding

    let create = |class: PCWSTR, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: u32| -> HWND {
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
        SendMessageW(child, WM_SETFONT, WPARAM(UI_FONT.0 as usize), LPARAM(1));
        child
    };

    let create_section = |text: &str, x: i32, y: i32, w: i32, id: u32| {
        let ctrl = create(w!("STATIC"), text, 0, x, y, w, 18, id);
        SendMessageW(ctrl, WM_SETFONT, WPARAM(SECTION_FONT.0 as usize), LPARAM(1));
    };

    // Owner-draw push button
    let create_btn = |text: &str, x: i32, y: i32, w: i32, h: i32, id: u32| {
        create(w!("BUTTON"), text, BS_OWNERDRAW as u32, x, y, w, h, id);
    };

    let mut y = pad;
    let right_edge = pad + client_w;  // right side of usable area
    let edit_w = 70;
    let btn_change_w = 85;

    // ── Display section ──
    create_section("DISPLAY", pad, y, client_w, IDC_SECTION_DISPLAY);
    y += 26;

    create(w!("BUTTON"), "Bar Mode", BS_AUTOCHECKBOX as u32, pad + inner_pad, y, 120, 22, IDC_BARS);
    y += 32;

    if settings.bars {
        SendDlgItemMessageW(hwnd, IDC_BARS as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));
    }

    // ── Timing section ──
    create_section("TIMING", pad, y, client_w, IDC_SECTION_TIMING);
    y += 26;

    create(w!("STATIC"), "Refresh interval (ms)", 0, pad + inner_pad, y + 2, 180, 20, IDC_SLEEP_LABEL);
    create(w!("EDIT"), &settings.sleep_time_ms.to_string(), WS_BORDER.0 | ES_NUMBER as u32, right_edge - edit_w, y, edit_w, 24, IDC_SLEEP_EDIT);
    y += 32;

    create(w!("STATIC"), "Bar width multiplier", 0, pad + inner_pad, y + 2, 180, 20, IDC_STEP_LABEL);
    create(w!("EDIT"), &settings.step_multiplier.to_string(), WS_BORDER.0 | ES_NUMBER as u32, right_edge - edit_w, y, edit_w, 24, IDC_STEP_EDIT);
    y += 38;

    // ── Colors section ──
    create_section("COLORS", pad, y, client_w, IDC_SECTION_COLORS);
    y += 26;

    // Top color row
    create(w!("STATIC"), "", 0, pad + inner_pad, y + 2, 24, 20, IDC_TOP_COLOR_SWATCH);
    create(w!("STATIC"), "Top gradient", 0, pad + inner_pad + 32, y + 2, 120, 20, IDC_TOP_COLOR_LABEL);
    create_btn("Change...", right_edge - btn_change_w, y, btn_change_w, 24, IDC_TOP_COLOR_BTN);
    y += 30;

    // Bottom color row
    create(w!("STATIC"), "", 0, pad + inner_pad, y + 2, 24, 20, IDC_BOTTOM_COLOR_SWATCH);
    create(w!("STATIC"), "Bottom gradient", 0, pad + inner_pad + 32, y + 2, 120, 20, IDC_BOTTOM_COLOR_LABEL);
    create_btn("Change...", right_edge - btn_change_w, y, btn_change_w, 24, IDC_BOTTOM_COLOR_BTN);
    y += 30;

    // Peak color row
    create(w!("STATIC"), "", 0, pad + inner_pad, y + 2, 24, 20, IDC_PEAK_COLOR_SWATCH);
    create(w!("STATIC"), "Peak line", 0, pad + inner_pad + 32, y + 2, 120, 20, IDC_PEAK_COLOR_LABEL);
    create_btn("Change...", right_edge - btn_change_w, y, btn_change_w, 24, IDC_PEAK_COLOR_BTN);
    y += 38;

    // ── Advanced section ──
    create_section("SPECTRUM", pad, y, client_w, IDC_SECTION_ADVANCED);
    y += 26;

    // Window type: radio group
    create(w!("STATIC"), "Window", 0, pad + inner_pad, y + 2, 55, 20, IDC_WINDOW_LABEL);
    let radio_first = 0x0009u32 | WS_GROUP.0; // BS_AUTORADIOBUTTON | WS_GROUP
    let radio = 0x0009u32;
    create(w!("BUTTON"), "Hann", radio_first, pad + inner_pad + 60, y, 65, 20, IDC_WINDOW_HANN);
    create(w!("BUTTON"), "Hamming", radio, pad + inner_pad + 130, y, 80, 20, IDC_WINDOW_HAMMING);
    create(w!("BUTTON"), "Blackman", radio, pad + inner_pad + 215, y, 85, 20, IDC_WINDOW_BH);
    y += 26;

    // Bin merge: radio group
    create(w!("STATIC"), "Merge", 0, pad + inner_pad, y + 2, 55, 20, IDC_MERGE_LABEL);
    let radio_first2 = 0x0009u32 | WS_GROUP.0;
    create(w!("BUTTON"), "Max", radio_first2, pad + inner_pad + 60, y, 65, 20, IDC_MERGE_MAX);
    create(w!("BUTTON"), "Average", radio, pad + inner_pad + 130, y, 80, 20, IDC_MERGE_AVG);
    y += 26;

    // Cutoff + Gain + Log spread on one row
    create(w!("STATIC"), "Cutoff (Hz)", 0, pad + inner_pad, y + 2, 80, 20, IDC_CUTOFF_LABEL);
    create(w!("EDIT"), &settings.freq_cutoff_hz.to_string(), WS_BORDER.0 | ES_NUMBER as u32, pad + inner_pad + 85, y, 55, 22, IDC_CUTOFF_EDIT);
    create(w!("STATIC"), "Gain", 0, pad + inner_pad + 155, y + 2, 35, 20, IDC_GAIN_LABEL);
    create(w!("EDIT"), &format!("{:.1}", settings.gain), WS_BORDER.0, pad + inner_pad + 195, y, 45, 22, IDC_GAIN_EDIT);
    create(w!("BUTTON"), "Log spread", BS_AUTOCHECKBOX as u32, pad + inner_pad + 260, y, 100, 20, IDC_LOG_SPREAD);
    y += 32;

    // Set radio + checkbox states
    let win_id = match settings.window_type {
        crate::config::WindowType::Hann => IDC_WINDOW_HANN,
        crate::config::WindowType::Hamming => IDC_WINDOW_HAMMING,
        crate::config::WindowType::BlackmanHarris => IDC_WINDOW_BH,
    };
    SendDlgItemMessageW(hwnd, win_id as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));

    let merge_id = match settings.bin_merge {
        crate::config::BinMergeMode::Max => IDC_MERGE_MAX,
        crate::config::BinMergeMode::Average => IDC_MERGE_AVG,
    };
    SendDlgItemMessageW(hwnd, merge_id as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));

    if settings.log_spread {
        SendDlgItemMessageW(hwnd, IDC_LOG_SPREAD as i32, BM_SETCHECK, WPARAM(1), LPARAM(0));
    }

    // ── Action buttons — right-aligned ──
    y += 8; // space above buttons
    let btn_count = 4;
    let btn_gap = 10;
    let btn_h = 32;
    let btn_w = (client_w - btn_gap * (btn_count - 1)) / btn_count;
    let btn_x = pad;

    create_btn("Apply", btn_x, y, btn_w, btn_h, IDC_APPLY);
    create_btn("Restart", btn_x + (btn_w + btn_gap), y, btn_w, btn_h, IDC_RESTART);
    create_btn("Reset", btn_x + (btn_w + btn_gap) * 2, y, btn_w, btn_h, IDC_RESET);
    create_btn("Close", btn_x + (btn_w + btn_gap) * 3, y, btn_w, btn_h, IDC_CLOSE);
    y += btn_h + 8;

    // Preview area — plain static, GL renders directly into it
    create(w!("STATIC"), "", 0, pad, y, client_w, PREVIEW_HEIGHT, IDC_PREVIEW);
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


fn set_edit_u32(hwnd: HWND, id: u32, value: u32) {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id as i32).unwrap_or_default();
        let text = value.to_string();
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        SetWindowTextW(ctrl, PCWSTR(wide.as_ptr()));
    }
}

fn get_edit_f32(hwnd: HWND, id: u32) -> f32 {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id as i32).unwrap_or_default();
        let mut buf = [0u16; 16];
        let len = GetWindowTextW(ctrl, &mut buf);
        let s = String::from_utf16_lossy(&buf[..len as usize]);
        s.parse().unwrap_or(6.0)
    }
}

fn set_edit_f32(hwnd: HWND, id: u32, value: f32) {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id as i32).unwrap_or_default();
        let text = format!("{:.1}", value);
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        SetWindowTextW(ctrl, PCWSTR(wide.as_ptr()));
    }
}

fn set_checked(hwnd: HWND, id: u32, checked: bool) {
    unsafe {
        SendDlgItemMessageW(
            hwnd, id as i32, BM_SETCHECK,
            WPARAM(if checked { 1 } else { 0 }), LPARAM(0),
        );
    }
}

fn sync_controls_from_settings(hwnd: HWND, settings: &Settings) {
    set_checked(hwnd, IDC_BARS, settings.bars);
    set_edit_u32(hwnd, IDC_SLEEP_EDIT, settings.sleep_time_ms);
    set_edit_u32(hwnd, IDC_STEP_EDIT, settings.step_multiplier);
    set_edit_u32(hwnd, IDC_CUTOFF_EDIT, settings.freq_cutoff_hz);
    set_edit_f32(hwnd, IDC_GAIN_EDIT, settings.gain);
    set_checked(hwnd, IDC_LOG_SPREAD, settings.log_spread);

    set_checked(hwnd, IDC_WINDOW_HANN, settings.window_type == crate::config::WindowType::Hann);
    set_checked(hwnd, IDC_WINDOW_HAMMING, settings.window_type == crate::config::WindowType::Hamming);
    set_checked(hwnd, IDC_WINDOW_BH, settings.window_type == crate::config::WindowType::BlackmanHarris);
    set_checked(hwnd, IDC_MERGE_MAX, settings.bin_merge == crate::config::BinMergeMode::Max);
    set_checked(hwnd, IDC_MERGE_AVG, settings.bin_merge == crate::config::BinMergeMode::Average);
}
/// Enable immersive dark mode title bar and set border/caption colors.
unsafe fn enable_dark_mode(hwnd: HWND) {
    // DWMWA_USE_IMMERSIVE_DARK_MODE = 20
    let dark: BOOL = TRUE;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWINDOWATTRIBUTE(20),
        &dark as *const BOOL as *const _,
        std::mem::size_of::<BOOL>() as u32,
    );

    // DWMWA_BORDER_COLOR = 34 — set border to match our background
    let border_color: u32 = BG_COLOR;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWINDOWATTRIBUTE(34),
        &border_color as *const u32 as *const _,
        std::mem::size_of::<u32>() as u32,
    );

    // DWMWA_CAPTION_COLOR = 35 — set caption/title bar color
    let caption_color: u32 = BG_COLOR;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWINDOWATTRIBUTE(35),
        &caption_color as *const u32 as *const _,
        std::mem::size_of::<u32>() as u32,
    );

    // DWMWA_TEXT_COLOR = 36 — set title bar text color
    let text_color: u32 = TEXT_COLOR;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWINDOWATTRIBUTE(36),
        &text_color as *const u32 as *const _,
        std::mem::size_of::<u32>() as u32,
    );
}

fn is_checked(hwnd: HWND, id: u32) -> bool {
    unsafe {
        let result = SendDlgItemMessageW(hwnd, id as i32, BM_GETCHECK, WPARAM(0), LPARAM(0));
        result.0 == 1
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
        WM_ERASEBKGND => {
            // Dark background
            let hdc = HDC(wparam.0 as *mut _);
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc);
            FillRect(hdc, &rc, BG_BRUSH);
            LRESULT(1)
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

            // Color swatches
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

            // Section headers get accent color
            match id {
                IDC_SECTION_DISPLAY | IDC_SECTION_TIMING | IDC_SECTION_COLORS => {
                    SetTextColor(hdc, COLORREF(ACCENT_COLOR));
                    SetBkColor(hdc, COLORREF(BG_COLOR));
                    return LRESULT(BG_BRUSH.0 as isize);
                }
                _ => {}
            }

            // All other static text: light text on dark bg
            SetTextColor(hdc, COLORREF(TEXT_COLOR));
            SetBkColor(hdc, COLORREF(BG_COLOR));
            LRESULT(BG_BRUSH.0 as isize)
        }
        WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            SetTextColor(hdc, COLORREF(TEXT_COLOR));
            SetBkColor(hdc, COLORREF(SURFACE_COLOR));
            LRESULT(SURFACE_BRUSH.0 as isize)
        }
        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as *mut _);
            SetTextColor(hdc, COLORREF(TEXT_COLOR));
            SetBkColor(hdc, COLORREF(OVERLAY_COLOR));
            LRESULT(OVERLAY_BRUSH.0 as isize)
        }
        WM_DRAWITEM => {
            let dis = &*(lparam.0 as *const DRAWITEMSTRUCT);
            let id = dis.CtlID;
            let hdc = dis.hDC;
            let rc = dis.rcItem;

            let is_pressed = (dis.itemState.0 & 0x0001) != 0; // ODS_SELECTED
            let is_focus = (dis.itemState.0 & 0x0010) != 0;   // ODS_FOCUS

            // Choose colors: "Apply" gets accent, others get surface
            let (bg, text, border) = if id == IDC_APPLY {
                if is_pressed {
                    (ACCENT_HOVER, BG_COLOR, ACCENT_COLOR)
                } else {
                    (ACCENT_COLOR, BG_COLOR, ACCENT_COLOR)
                }
            } else if is_pressed {
                (OVERLAY_COLOR, TEXT_COLOR, BORDER_COLOR)
            } else {
                (SURFACE_COLOR, TEXT_COLOR, BORDER_COLOR)
            };

            // Border
            let border_brush = CreateSolidBrush(COLORREF(border));
            FrameRect(hdc, &rc, border_brush);
            DeleteObject(border_brush);

            // Fill (inset by 1 for border)
            let inner = RECT {
                left: rc.left + 1,
                top: rc.top + 1,
                right: rc.right - 1,
                bottom: rc.bottom - 1,
            };
            let fill_brush = CreateSolidBrush(COLORREF(bg));
            FillRect(hdc, &inner, fill_brush);
            DeleteObject(fill_brush);

            // Text
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(text));
            let old_font = SelectObject(hdc, HGDIOBJ(UI_FONT.0));
            let mut text_rc = rc;
            let mut buf = [0u16; 64];
            let len = GetWindowTextW(dis.hwndItem, &mut buf);
            DrawTextW(hdc, &mut buf[..len as usize], &mut text_rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            SelectObject(hdc, old_font);

            // Focus dotted rect
            if is_focus {
                let focus_rc = RECT {
                    left: rc.left + 3,
                    top: rc.top + 3,
                    right: rc.right - 3,
                    bottom: rc.bottom - 3,
                };
                DrawFocusRect(hdc, &focus_rc);
            }

            LRESULT(1)
        }
        WM_TIMER => {
            if wparam.0 == PREVIEW_TIMER_ID {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if state_ptr != 0 {
                    let state = &mut *(state_ptr as *mut UiState);

                    // Init GL preview renderer on first tick
                    if state.preview_renderer.is_none() {
                        let preview_hwnd = GetDlgItem(hwnd, IDC_PREVIEW as i32).unwrap_or_default();
                        if !preview_hwnd.is_invalid() {
                            let mut r = crate::renderer::Renderer::new(256);
                            if r.init_on_child(preview_hwnd).is_ok() {
                                state.preview_renderer = Some(r);
                            }
                        }
                    }

                    // Read shared falloff from taskbar renderer and draw
                    if let Some(ref mut renderer) = state.preview_renderer {
                        if let Ok(sf) = state.shared_falloff.lock() {
                            renderer.render_preview_from_shared(&state.local, &sf);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if state_ptr == 0 {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let state = &mut *(state_ptr as *mut UiState);
            let cmd = (wparam.0 & 0xFFFF) as u32;
            let notify = ((wparam.0 >> 16) & 0xFFFF) as u32;
            // BN_CLICKED = 0
            let is_click = notify == 0;

            match cmd {
                IDC_BARS if is_click => {
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
                    state.local.bars = is_checked(hwnd, IDC_BARS);
                    state.local.freq_cutoff_hz = get_edit_u32(hwnd, IDC_CUTOFF_EDIT);
                    state.local.log_spread = is_checked(hwnd, IDC_LOG_SPREAD);
                    state.local.gain = get_edit_f32(hwnd, IDC_GAIN_EDIT);

                    state.local.window_type = if is_checked(hwnd, IDC_WINDOW_HAMMING) {
                        crate::config::WindowType::Hamming
                    } else if is_checked(hwnd, IDC_WINDOW_BH) {
                        crate::config::WindowType::BlackmanHarris
                    } else {
                        crate::config::WindowType::Hann
                    };

                    state.local.bin_merge = if is_checked(hwnd, IDC_MERGE_AVG) {
                        crate::config::BinMergeMode::Average
                    } else {
                        crate::config::BinMergeMode::Max
                    };

                    *state.settings.lock().unwrap() = state.local.clone();
                    if let Err(e) = state.local.save() {
                        log::error!("Failed to save config: {e}");
                    }
                }
                IDC_RESET => {
                    state.local = state.settings.lock().unwrap().clone();
                    sync_controls_from_settings(hwnd, &state.local);
                    let _ = InvalidateRect(hwnd, None, true);
                }
                IDC_CLOSE => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                IDC_RESTART => {
                    (state.stop_fn)();
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    (state.start_fn)();
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
