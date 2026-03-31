use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

/// Find the main taskbar window (Shell_TrayWnd) and its task-list child.
pub struct TaskbarInfo {
    pub taskbar_hwnd: HWND,
    pub tasklist_hwnd: Option<HWND>,
}

impl TaskbarInfo {
    pub fn locate() -> Option<Self> {
        unsafe {
            let taskbar_hwnd = match FindWindowW(w!("Shell_TrayWnd"), None) {
                Ok(h) if !h.0.is_null() => h,
                _ => {
                    log::error!("Shell_TrayWnd not found");
                    return None;
                }
            };

            let mut tasklist_hwnd: Option<HWND> = None;
            let ptr: *mut Option<HWND> = &mut tasklist_hwnd;

            let _ = EnumChildWindows(
                taskbar_hwnd,
                Some(find_child_proc),
                LPARAM(ptr as isize),
            );

            Some(Self {
                taskbar_hwnd,
                tasklist_hwnd,
            })
        }
    }

    /// Get the outer rect (full taskbar) and inner rect (task list area or full).
    pub fn get_rects(&self, full_taskbar: bool) -> Option<(RECT, RECT)> {
        unsafe {
            let mut outer = RECT::default();
            if GetWindowRect(self.taskbar_hwnd, &mut outer).is_err() {
                return None;
            }

            let inner = if full_taskbar {
                outer
            } else if let Some(tl) = self.tasklist_hwnd {
                let mut r = RECT::default();
                if GetWindowRect(tl, &mut r).is_err() {
                    outer
                } else {
                    r
                }
            } else {
                outer
            };

            Some((outer, inner))
        }
    }
}

unsafe extern "system" fn find_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len > 0 {
        let text = String::from_utf16_lossy(&buf[..len as usize]);
        if text == "Running applications" {
            let ptr = lparam.0 as *mut Option<HWND>;
            *ptr = Some(hwnd);
            return BOOL(0); // stop enumeration
        }
    }
    BOOL(1)
}
