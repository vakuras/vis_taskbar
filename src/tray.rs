use crate::config::VisRgb;

use windows::Win32::Foundation::*;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// Tray icon manager.
pub struct TrayIcon {
    hwnd: HWND,
    icon_uid: u32,
}

pub const TRAY_MSG: u32 = WM_APP;
pub const CMD_SHOW_CONFIG: u32 = WM_APP + 1;
pub const CMD_HIDE_CONFIG: u32 = WM_APP + 2;
pub const CMD_START: u32 = WM_APP + 3;
pub const CMD_STOP: u32 = WM_APP + 4;
pub const CMD_EXIT: u32 = WM_APP + 5;

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Result<Self> {
        use windows::Win32::UI::Shell::*;
        unsafe {
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 459,
                uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE,
                uCallbackMessage: TRAY_MSG,
                // Load embedded icon (resource ID 1 set by winresource)
                hIcon: {
                    let hinstance: HINSTANCE = std::mem::transmute(GetModuleHandleW(None).unwrap_or_default());
                    LoadIconW(hinstance, PCWSTR(1 as *const u16))?
                },
                ..std::mem::zeroed()
            };

            let tip_bytes = b"vis_taskbar\0";
            for (i, &b) in tip_bytes.iter().enumerate() {
                if i < nid.szTip.len() {
                    nid.szTip[i] = b as u16;
                }
            }

            Shell_NotifyIconW(NIM_ADD, &nid).ok()?;

            Ok(Self {
                hwnd,
                icon_uid: 459,
            })
        }
    }

    pub fn remove(&self) {
        use windows::Win32::UI::Shell::*;
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: self.icon_uid,
                ..std::mem::zeroed()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    pub fn show_context_menu(&self, is_running: bool) {
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            let hmenu = CreatePopupMenu().unwrap();
            let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(0), CMD_SHOW_CONFIG as usize, w!("&Show Configuration"));
            if is_running {
                let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(0), CMD_STOP as usize, w!("S&top"));
            } else {
                let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(0), CMD_START as usize, w!("S&tart"));
            }
            let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(0), CMD_EXIT as usize, w!("&Exit"));

            let _ = SetForegroundWindow(self.hwnd);
            TrackPopupMenu(
                hmenu,
                TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                0,
                self.hwnd,
                None,
            );
            let _ = DestroyMenu(hmenu);
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Show Win32 color chooser dialog.
pub fn show_color_dialog(owner: HWND, current: VisRgb) -> VisRgb {
    unsafe {
        let mut custom_colors = [COLORREF(0); 16];
        let mut cc = CHOOSECOLORW {
            lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
            hwndOwner: owner,
            rgbResult: COLORREF(current.to_colorref()),
            lpCustColors: custom_colors.as_mut_ptr(),
            Flags: CC_FULLOPEN | CC_RGBINIT,
            ..std::mem::zeroed()
        };

        if ChooseColorW(&mut cc).as_bool() {
            VisRgb::from_colorref(cc.rgbResult.0)
        } else {
            current
        }
    }
}
