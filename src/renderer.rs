use crate::config::Settings;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::OpenGL::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use std::sync::{Arc, Mutex};

const VIS_CLASS: PCWSTR = w!("VIS_TASKBAR_CLASS");

/// Data passed through the render loop.
pub struct SpectrumFrame {
    /// 0..=255 values, first half = left channel, second half = right channel.
    pub values: Vec<u8>,
}

/// The OpenGL renderer that draws the spectrum on a transparent taskbar overlay.
pub struct Renderer {
    hwnd: HWND,
    hdc: HDC,
    hglrc: HGLRC,
    data_size: usize,
    vis_falloff: Vec<i16>,
    vis_peak_falloff: Vec<i16>,
    started: bool,
}

impl Renderer {
    pub fn new(data_size: usize) -> Self {
        Self {
            hwnd: HWND::default(),
            hdc: HDC::default(),
            hglrc: HGLRC::default(),
            data_size,
            vis_falloff: vec![0i16; data_size],
            vis_peak_falloff: vec![0i16; data_size],
            started: false,
        }
    }

    /// Create the overlay window and initialize OpenGL context.
    /// Must be called from the render thread.
    pub fn init_window(&mut self, taskbar_rect: &RECT) -> Result<()> {
        unsafe {
            let hinstance: HINSTANCE = std::mem::transmute(GetModuleHandleW(None)?);

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW)?,
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: VIS_CLASS,
                ..std::mem::zeroed()
            };

            let atom = RegisterClassExW(&wc);
            if atom == 0 {
                return Err(Error::from_win32());
            }

            let w = taskbar_rect.right - taskbar_rect.left;
            let h = taskbar_rect.bottom - taskbar_rect.top;

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                VIS_CLASS,
                w!("VIS_TASKBAR"),
                WS_POPUP,
                taskbar_rect.left,
                taskbar_rect.top,
                w,
                h,
                HWND::default(),
                HMENU::default(),
                hinstance,
                None,
            )?;

            ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = UpdateWindow(hwnd);

            self.hwnd = hwnd;
            self.init_gl()?;
            self.started = true;

            Ok(())
        }
    }

    fn init_gl(&mut self) -> Result<()> {
        unsafe {
            let hdc = GetDC(self.hwnd);
            if hdc.is_invalid() {
                return Err(Error::from_win32());
            }

            let pfd = PIXELFORMATDESCRIPTOR {
                nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
                nVersion: 1,
                dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
                iPixelType: PFD_TYPE_RGBA,
                cColorBits: 32,
                cDepthBits: 16,
                iLayerType: PFD_MAIN_PLANE.0 as u8,
                ..Default::default()
            };

            let pixel_format = ChoosePixelFormat(hdc, &pfd);
            if pixel_format == 0 {
                return Err(Error::from_win32());
            }

            if SetPixelFormat(hdc, pixel_format, &pfd).is_err() {
                return Err(Error::from_win32());
            }

            let hglrc = wglCreateContext(hdc)?;
            wglMakeCurrent(hdc, hglrc)?;

            glClearColor(0.0, 0.0, 0.0, 0.0);
            glClearDepth(1.0);
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

            self.hdc = hdc;
            self.hglrc = hglrc;

            Ok(())
        }
    }

    /// Update the window position to match the taskbar.
    pub fn update_position(&self, outer: &RECT) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOP,
                outer.left,
                outer.top,
                outer.right - outer.left,
                outer.bottom - outer.top,
                SET_WINDOW_POS_FLAGS(0),
            );
        }
    }

    /// Apply falloff to stored values and render a frame.
    pub fn render(
        &mut self,
        new_values: Option<&[u8]>,
        outer: &RECT,
        inner: &RECT,
        settings: &Settings,
    ) {
        self.update_falloff(new_values);
        self.render_gl(outer, inner, settings);
    }

    /// GL rendering (assumes GL context is current).
    fn render_gl(
        &self,
        outer: &RECT,
        inner: &RECT,
        settings: &Settings,
    ) {
        let height = inner.bottom - inner.top;
        let width = inner.right - inner.left;
        let outer_w = outer.right - outer.left;
        let outer_h = outer.bottom - outer.top;

        let half_data = self.data_size / 2;

        unsafe {
            glMatrixMode(GL_PROJECTION);
            glLoadIdentity();
            glOrtho(0.0, outer_w as f64, outer_h as f64, 0.0, 0.0, 1.0);
            glDisable(GL_DEPTH_TEST);
            glMatrixMode(GL_MODELVIEW);
            glLoadIdentity();
            glTranslatef(0.375, 0.375, 0.0);
            glClearColor(0.0, 0.0, 0.0, 0.0);
            glClear(GL_COLOR_BUFFER_BIT);

            if width <= 0 || self.data_size == 0 {
                let _ = SwapBuffers(self.hdc);
                return;
            }

            let step = ((width as f64 / self.data_size as f64).ceil() as i32)
                * settings.step_multiplier as i32;
            let center = (inner.left - outer.left) + width / 2;

            // Left spectrum
            let mut target_left = ((center - (inner.left - outer.left)) / step.max(1) - 1) as usize;
            target_left = target_left.min(half_data - 1);
            if settings.bars {
                target_left += 1;
            }

            self.draw_bars(
                &settings.color_top,
                &settings.color_bottom,
                center,
                step,
                height,
                target_left,
                0,
                true,
                settings.bars,
            );
            self.draw_peaks(
                &settings.color_peaks,
                center,
                step,
                height,
                target_left,
                0,
                true,
            );

            // Right spectrum
            let mut target_right =
                (((inner.right - outer.left) - center) / step.max(1) + 1) as usize;
            target_right = target_right.min(half_data - 1);
            if settings.bars {
                target_right += 1;
            }

            self.draw_bars(
                &settings.color_top,
                &settings.color_bottom,
                center,
                step,
                height,
                target_right,
                half_data,
                false,
                settings.bars,
            );
            self.draw_peaks(
                &settings.color_peaks,
                center,
                step,
                height,
                target_right,
                half_data,
                false,
            );

            let _ = SwapBuffers(self.hdc);
        }
    }

    fn draw_bars(
        &self,
        color_top: &crate::config::VisRgb,
        color_bottom: &crate::config::VisRgb,
        center: i32,
        step: i32,
        height: i32,
        count: usize,
        offset: usize,
        is_left: bool,
        bars_mode: bool,
    ) {
        unsafe {
            glBegin(GL_QUADS);
            for i in 0..count {
                let val1 = self.vis_falloff[i + offset] as i32 * height / 255;
                let val2 = if bars_mode {
                    val1
                } else if i + 1 + offset < self.vis_falloff.len() {
                    self.vis_falloff[i + 1 + offset] as i32 * height / 255
                } else {
                    val1
                };

                if is_left {
                    glColor3f(color_top.r, color_top.g, color_top.b);
                    glVertex2i(center - (i as i32 + 1) * step, height - val2);
                    glVertex2i(center - i as i32 * step, height - val1);
                    glColor3f(color_bottom.r, color_bottom.g, color_bottom.b);
                    glVertex2i(center - i as i32 * step, height);
                    glVertex2i(center - (i as i32 + 1) * step, height);
                } else {
                    glColor3f(color_top.r, color_top.g, color_top.b);
                    glVertex2i(center + i as i32 * step, height - val1);
                    glVertex2i(center + (i as i32 + 1) * step, height - val2);
                    glColor3f(color_bottom.r, color_bottom.g, color_bottom.b);
                    glVertex2i(center + (i as i32 + 1) * step, height);
                    glVertex2i(center + i as i32 * step, height);
                }
            }
            glEnd();
        }
    }

    fn draw_peaks(
        &self,
        color: &crate::config::VisRgb,
        center: i32,
        step: i32,
        height: i32,
        count: usize,
        offset: usize,
        is_left: bool,
    ) {
        unsafe {
            glColor3f(color.r, color.g, color.b);
            glBegin(GL_LINE_STRIP);
            for i in 0..count {
                let val = self.vis_peak_falloff[i + offset] as i32 * height / 255;
                if is_left {
                    glVertex2i(center - i as i32 * step, height - val);
                } else {
                    glVertex2i(center + i as i32 * step, height - val);
                }
            }
            glEnd();
        }
    }

    pub fn cleanup(&mut self) {
        unsafe {
            let _ = wglMakeCurrent(HDC::default(), HGLRC::default());
            if !self.hglrc.is_invalid() {
                let _ = wglDeleteContext(self.hglrc);
            }
            if !self.hdc.is_invalid() {
                ReleaseDC(self.hwnd, self.hdc);
            }
            if !self.hwnd.is_invalid() {
                let _ = DestroyWindow(self.hwnd);
            }
            let _ = UnregisterClassW(VIS_CLASS, None);
            self.started = false;
        }
    }

    #[allow(dead_code)]
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Initialize an OpenGL context on an existing child HWND (e.g. a static control).
    pub fn init_on_child(&mut self, hwnd: HWND) -> Result<()> {
        self.hwnd = hwnd;
        self.init_gl()?;
        self.started = true;
        Ok(())
    }

    /// Render preview using shared falloff data from the taskbar renderer.
    /// This ensures the preview looks identical to the taskbar.
    pub fn render_preview_from_shared(&mut self, settings: &Settings, shared: &SharedFalloff) {
        unsafe {
            wglMakeCurrent(self.hdc, self.hglrc);

            let mut rc = RECT::default();
            GetClientRect(self.hwnd, &mut rc);
            let preview_w = rc.right - rc.left;
            let preview_h = rc.bottom - rc.top;
            if preview_w <= 0 || preview_h <= 0 { return; }

            glViewport(0, 0, preview_w, preview_h);

            // Copy shared falloff
            let len = self.data_size.min(shared.falloff.len());
            self.vis_falloff[..len].copy_from_slice(&shared.falloff[..len]);
            let len = self.data_size.min(shared.peaks.len());
            self.vis_peak_falloff[..len].copy_from_slice(&shared.peaks[..len]);

            // Fake the width to match taskbar (same bar pixel width).
            // Use actual preview height so bars fill vertically.
            let fake_w = 1920;
            let offset_x = (fake_w - preview_w) / 2;

            glMatrixMode(GL_PROJECTION);
            glLoadIdentity();
            glOrtho(
                offset_x as f64,
                (offset_x + preview_w) as f64,
                preview_h as f64,  // actual height — bars fill the preview
                0.0,
                0.0,
                1.0,
            );
            glDisable(GL_DEPTH_TEST);
            glMatrixMode(GL_MODELVIEW);
            glLoadIdentity();
            glTranslatef(0.375, 0.375, 0.0);
            // Match dialog background #1e2028
            glClearColor(0.118, 0.125, 0.157, 1.0);
            glClear(GL_COLOR_BUFFER_BIT);
            let half_data = self.data_size / 2;
            let height = preview_h; // use real height for bar scaling
            let step = ((fake_w as f64 / self.data_size as f64).ceil() as i32)
                * settings.step_multiplier as i32;
            let center = fake_w / 2;

            let mut target_left = ((center / step.max(1)) - 1) as usize;
            target_left = target_left.min(half_data - 1);
            if settings.bars { target_left += 1; }

            self.draw_bars(
                &settings.color_top, &settings.color_bottom,
                center, step, height, target_left, 0, true, settings.bars,
            );
            self.draw_peaks(
                &settings.color_peaks, center, step, height, target_left, 0, true,
            );

            let mut target_right = ((center / step.max(1)) + 1) as usize;
            target_right = target_right.min(half_data - 1);
            if settings.bars { target_right += 1; }

            self.draw_bars(
                &settings.color_top, &settings.color_bottom,
                center, step, height, target_right, half_data, false, settings.bars,
            );
            self.draw_peaks(
                &settings.color_peaks, center, step, height, target_right, half_data, false,
            );

            let _ = SwapBuffers(self.hdc);
        }
    }

    /// Update falloff from new values without rendering.
    pub fn update_falloff(&mut self, new_values: Option<&[u8]>) {
        for i in 0..self.data_size {
            let value = new_values.map_or(0, |v| v.get(i).copied().unwrap_or(0)) as i16;

            if new_values.is_some() && self.vis_falloff[i] < value {
                self.vis_falloff[i] = value;
            } else {
                self.vis_falloff[i] = (self.vis_falloff[i] - 7).max(0);
            }

            if new_values.is_some() && self.vis_peak_falloff[i] < value {
                self.vis_peak_falloff[i] = value;
            } else {
                self.vis_peak_falloff[i] = (self.vis_peak_falloff[i] - 2).max(0);
            }
        }
    }

    pub fn process_messages(&self) -> bool {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return false;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            true
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Shared falloff data — written by taskbar renderer, read by preview.
pub struct SharedFalloff {
    pub falloff: Vec<i16>,
    pub peaks: Vec<i16>,
}

/// Run the render loop on the current thread.
pub fn render_loop(
    frame_rx: crossbeam_channel::Receiver<SpectrumFrame>,
    settings: Arc<Mutex<Settings>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    taskbar_info: &crate::taskbar::TaskbarInfo,
    shared_falloff: Arc<Mutex<SharedFalloff>>,
) {
    let (outer, _inner) = match taskbar_info.get_rects() {
        Some(r) => r,
        None => {
            log::error!("Failed to get taskbar rects for renderer init");
            return;
        }
    };

    // Determine data size: we expect 2 * bin_size (left + right)
    // Default FFT_SIZE=256, bin_size=128, data_size=256
    let data_size = 256;
    let mut renderer = Renderer::new(data_size);

    if let Err(e) = renderer.init_window(&outer) {
        log::error!("Failed to init render window: {e}");
        return;
    }

    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        if !renderer.process_messages() {
            break;
        }

        let settings = settings.lock().unwrap().clone();

        // Try to receive a new frame (non-blocking)
        let frame = frame_rx.try_recv().ok();
        let new_values = frame.as_ref().map(|f| f.values.as_slice());

        // Update taskbar position
        if let Some((outer, inner)) = taskbar_info.get_rects() {
            renderer.update_position(&outer);
            renderer.render(new_values, &outer, &inner, &settings);

            // Copy falloff to shared for preview
            if let Ok(mut sf) = shared_falloff.lock() {
                sf.falloff.clear();
                sf.falloff.extend_from_slice(&renderer.vis_falloff);
                sf.peaks.clear();
                sf.peaks.extend_from_slice(&renderer.vis_peak_falloff);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(settings.sleep_time_ms as u64));
    }

    renderer.cleanup();
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
