use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

/// Find the main taskbar window (Shell_TrayWnd).
pub struct TaskbarInfo {
    pub taskbar_hwnd: HWND,
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

            Some(Self { taskbar_hwnd })
        }
    }

    /// Get the taskbar rect (used as both outer and inner).
    pub fn get_rects(&self) -> Option<(RECT, RECT)> {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(self.taskbar_hwnd, &mut rect).is_err() {
                return None;
            }
            Some((rect, rect))
        }
    }
}
