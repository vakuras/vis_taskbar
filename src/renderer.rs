use crate::config::Settings;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;
use windows::Foundation::Numerics::Matrix3x2;

use std::sync::{Arc, Mutex};

const VIS_CLASS: PCWSTR = w!("VIS_TASKBAR_CLASS");

/// Data passed through the render loop.
pub struct SpectrumFrame {
    /// 0..=255 values, first half = left channel, second half = right channel.
    pub values: Vec<u8>,
}

/// Direct2D renderer that draws the spectrum on a transparent taskbar overlay.
pub struct Renderer {
    hwnd: HWND,
    factory: Option<ID2D1Factory>,
    render_target: Option<ID2D1HwndRenderTarget>,
    data_size: usize,
    vis_falloff: Vec<i16>,
    vis_peak_falloff: Vec<i16>,
    started: bool,
}

impl Renderer {
    pub fn new(data_size: usize) -> Self {
        Self {
            hwnd: HWND::default(),
            factory: None,
            render_target: None,
            data_size,
            vis_falloff: vec![0i16; data_size],
            vis_peak_falloff: vec![0i16; data_size],
            started: false,
        }
    }

    /// Create the overlay window and initialize Direct2D.
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
            self.init_d2d(w, h)?;
            self.started = true;

            Ok(())
        }
    }

    fn init_d2d(&mut self, width: i32, height: i32) -> Result<()> {
        unsafe {
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;

            let render_props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                ..Default::default()
            };

            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                    width: width as u32,
                    height: height as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
            };

            let render_target = factory.CreateHwndRenderTarget(&render_props, &hwnd_props)?;
            render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);

            self.factory = Some(factory);
            self.render_target = Some(render_target);

            Ok(())
        }
    }

    /// Update the window position and resize the render target.
    pub fn update_position(&mut self, outer: &RECT) {
        unsafe {
            let w = (outer.right - outer.left) as u32;
            let h = (outer.bottom - outer.top) as u32;

            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOP,
                outer.left,
                outer.top,
                w as i32,
                h as i32,
                SET_WINDOW_POS_FLAGS(0),
            );

            if let Some(ref rt) = self.render_target {
                let _ = rt.Resize(&D2D_SIZE_U { width: w, height: h });
            }
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
        self.render_d2d(outer, inner, settings);
    }

    fn render_d2d(&self, outer: &RECT, inner: &RECT, settings: &Settings) {
        let Some(ref rt) = self.render_target else { return };
        let height = (inner.bottom - inner.top) as f32;
        let width = inner.right - inner.left;
        if width <= 0 || self.data_size == 0 { return; }

        let half_data = self.data_size / 2;
        let step = ((width as f64 / self.data_size as f64).ceil() as f32)
            * settings.step_multiplier as f32;
        let center = ((inner.left - outer.left) + width / 2) as f32;

        unsafe {
            rt.BeginDraw();
            rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

            // Left spectrum
            let mut target_left = ((center - (inner.left - outer.left) as f32) / step - 1.0) as usize;
            target_left = target_left.min(half_data - 1);
            if settings.bars { target_left += 1; }

            self.draw_bars_d2d(rt, settings, center, step, height, target_left, 0, true, settings.bars);
            self.draw_peaks_d2d(rt, settings, center, step, height, target_left, 0, true);

            // Right spectrum
            let mut target_right = (((inner.right - outer.left) as f32 - center) / step + 1.0) as usize;
            target_right = target_right.min(half_data - 1);
            if settings.bars { target_right += 1; }

            self.draw_bars_d2d(rt, settings, center, step, height, target_right, half_data, false, settings.bars);
            self.draw_peaks_d2d(rt, settings, center, step, height, target_right, half_data, false);

            let _ = rt.EndDraw(None, None);
        }
    }

    fn draw_bars_d2d(
        &self,
        rt: &ID2D1HwndRenderTarget,
        settings: &Settings,
        center: f32,
        step: f32,
        height: f32,
        count: usize,
        offset: usize,
        is_left: bool,
        bars_mode: bool,
    ) {
        let Some(ref factory) = self.factory else { return };
        unsafe {
            for i in 0..count {
                let val1 = self.vis_falloff[i + offset] as f32 * height / 255.0;
                let val2 = if bars_mode {
                    val1
                } else if i + 1 + offset < self.vis_falloff.len() {
                    self.vis_falloff[i + 1 + offset] as f32 * height / 255.0
                } else {
                    val1
                };
                if val1 <= 0.0 && val2 <= 0.0 { continue; }

                let (x1, x2) = if is_left {
                    (center - (i as f32 + 1.0) * step, center - i as f32 * step)
                } else {
                    (center + i as f32 * step, center + (i as f32 + 1.0) * step)
                };

                // For smooth fill, draw a quad (trapezoid) using path geometry
                // top-left = (x1, height - val2), top-right = (x2, height - val1)
                // bottom-right = (x2, height), bottom-left = (x1, height)
                let y_top_left = height - if is_left { val2 } else { val1 };
                let y_top_right = height - if is_left { val1 } else { val2 };
                let max_val = val1.max(val2);

                // Gradient: draw in segments (approximate gradient on the quad)
                let segments = 8.min(max_val as i32).max(1);
                for s in 0..segments {
                    let t = s as f32 / segments as f32;
                    let t1 = (s + 1) as f32 / segments as f32;

                    // Interpolate the top edge for this segment
                    let seg_top_l = y_top_left + (height - y_top_left) * t;
                    let seg_bot_l = y_top_left + (height - y_top_left) * t1;
                    let seg_top_r = y_top_right + (height - y_top_right) * t;
                    let seg_bot_r = y_top_right + (height - y_top_right) * t1;

                    let r = settings.color_top.r * (1.0 - t) + settings.color_bottom.r * t;
                    let g = settings.color_top.g * (1.0 - t) + settings.color_bottom.g * t;
                    let b = settings.color_top.b * (1.0 - t) + settings.color_bottom.b * t;

                    let color = D2D1_COLOR_F { r, g, b, a: 1.0 };
                    let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                    if bars_mode {
                        // Simple rectangle
                        rt.FillRectangle(
                            &D2D_RECT_F { left: x1, top: seg_top_l, right: x2, bottom: seg_bot_l },
                            &brush,
                        );
                    } else {
                        // Quad via path geometry
                        let geom: ID2D1PathGeometry = factory.CreatePathGeometry().unwrap();
                        let sink = geom.Open().unwrap();
                        sink.BeginFigure(
                            D2D_POINT_2F { x: x1, y: seg_top_l },
                            D2D1_FIGURE_BEGIN_FILLED,
                        );
                        sink.AddLine(D2D_POINT_2F { x: x2, y: seg_top_r });
                        sink.AddLine(D2D_POINT_2F { x: x2, y: seg_bot_r });
                        sink.AddLine(D2D_POINT_2F { x: x1, y: seg_bot_l });
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                        sink.Close().unwrap();
                        rt.FillGeometry(&geom, &brush, None);
                    }
                }
            }
        }
    }

    fn draw_peaks_d2d(
        &self,
        rt: &ID2D1HwndRenderTarget,
        settings: &Settings,
        center: f32,
        step: f32,
        height: f32,
        count: usize,
        offset: usize,
        is_left: bool,
    ) {
        unsafe {
            let color = D2D1_COLOR_F {
                r: settings.color_peaks.r,
                g: settings.color_peaks.g,
                b: settings.color_peaks.b,
                a: 1.0,
            };
            let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

            for i in 0..count.saturating_sub(1) {
                let val1 = self.vis_peak_falloff[i + offset] as f32 * height / 255.0;
                let val2 = self.vis_peak_falloff[i + 1 + offset] as f32 * height / 255.0;

                let (x1, x2) = if is_left {
                    (center - i as f32 * step, center - (i as f32 + 1.0) * step)
                } else {
                    (center + i as f32 * step, center + (i as f32 + 1.0) * step)
                };

                rt.DrawLine(
                    D2D_POINT_2F { x: x1, y: height - val1 },
                    D2D_POINT_2F { x: x2, y: height - val2 },
                    &brush,
                    1.0,
                    None,
                );
            }
        }
    }

    pub fn cleanup(&mut self) {
        self.render_target = None;
        self.factory = None;
        unsafe {
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

    /// Initialize Direct2D on an existing child HWND (for preview).
    pub fn init_on_child(&mut self, hwnd: HWND) -> Result<()> {
        unsafe {
            self.hwnd = hwnd;
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc);
            self.init_d2d(rc.right - rc.left, rc.bottom - rc.top)?;
            self.started = true;
            Ok(())
        }
    }

    /// Render preview using shared falloff data from the taskbar renderer.
    pub fn render_preview_from_shared(&mut self, settings: &Settings, shared: &SharedFalloff) {
        let Some(ref rt) = self.render_target else { return };
        unsafe {
            let mut rc = RECT::default();
            GetClientRect(self.hwnd, &mut rc);
            let preview_w = rc.right - rc.left;
            let preview_h = rc.bottom - rc.top;
            if preview_w <= 0 || preview_h <= 0 { return; }

            let _ = rt.Resize(&D2D_SIZE_U {
                width: preview_w as u32,
                height: preview_h as u32,
            });

            // Copy shared falloff
            let len = self.data_size.min(shared.falloff.len());
            self.vis_falloff[..len].copy_from_slice(&shared.falloff[..len]);
            let len = self.data_size.min(shared.peaks.len());
            self.vis_peak_falloff[..len].copy_from_slice(&shared.peaks[..len]);

            // Fake width for consistent bar size, real height
            let fake_w = 1920;
            let offset_x = (fake_w - preview_w) / 2;
            let half_data = self.data_size / 2;
            let height = preview_h as f32;
            let step = ((fake_w as f64 / self.data_size as f64).ceil() as f32)
                * settings.step_multiplier as f32;
            let center = (fake_w / 2) as f32;

            // Offset transform so only center slice is visible
            rt.BeginDraw();
            rt.SetTransform(&Matrix3x2 {
                M11: 1.0, M12: 0.0,
                M21: 0.0, M22: 1.0,
                M31: -(offset_x as f32), M32: 0.0,
            });

            // Match dialog background #1e2028
            rt.Clear(Some(&D2D1_COLOR_F { r: 0.118, g: 0.125, b: 0.157, a: 1.0 }));

            let mut target_left = ((center / step) - 1.0) as usize;
            target_left = target_left.min(half_data - 1);
            if settings.bars { target_left += 1; }

            self.draw_bars_d2d(rt, settings, center, step, height, target_left, 0, true, settings.bars);
            self.draw_peaks_d2d(rt, settings, center, step, height, target_left, 0, true);

            let mut target_right = ((center / step) + 1.0) as usize;
            target_right = target_right.min(half_data - 1);
            if settings.bars { target_right += 1; }

            self.draw_bars_d2d(rt, settings, center, step, height, target_right, half_data, false, settings.bars);
            self.draw_peaks_d2d(rt, settings, center, step, height, target_right, half_data, false);

            // Reset transform
            rt.SetTransform(&Matrix3x2::identity());

            let _ = rt.EndDraw(None, None);
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

/// Shared falloff data written by taskbar renderer, read by preview.
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

        let frame = frame_rx.try_recv().ok();
        let new_values = frame.as_ref().map(|f| f.values.as_slice());

        if let Some((outer, inner)) = taskbar_info.get_rects() {
            renderer.update_position(&outer);
            renderer.render(new_values, &outer, &inner, &settings);

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