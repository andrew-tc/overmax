use super::WindowRect;

pub struct WindowTracker {
    title: Vec<u16>,
}

impl WindowTracker {
    pub fn new(title: &str) -> Self {
        Self {
            title: encode_wide(title),
        }
    }

    pub fn game_rect(&self) -> Option<WindowRect> {
        self.find_hwnd().and_then(client_rect_for_hwnd)
    }

    pub fn is_foreground(&self) -> bool {
        let Some(hwnd) = self.find_hwnd() else {
            return false;
        };
        let fg = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        if fg == hwnd {
            return true;
        }

        let mut fg_pid = 0u32;
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(fg, &mut fg_pid);
            let my_pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
            fg_pid == my_pid
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        let Some(hwnd) = self.find_hwnd() else {
            return false;
        };
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetWindowLongW, GWL_STYLE, WS_POPUP,
            };

            let style = GetWindowLongW(hwnd, GWL_STYLE);
            if (style as u32 & WS_POPUP) == 0 {
                return false;
            }

            let mut rect = windows_sys::Win32::Foundation::RECT::default();
            if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect) == 0 {
                return false;
            }

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if monitor.is_null() {
                return false;
            }

            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                rcMonitor: windows_sys::Win32::Foundation::RECT::default(),
                rcWork: windows_sys::Win32::Foundation::RECT::default(),
                dwFlags: 0,
            };

            if GetMonitorInfoW(monitor, &mut monitor_info) == 0 {
                return false;
            }

            let win_width = rect.right - rect.left;
            let win_height = rect.bottom - rect.top;
            let mon_width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
            let mon_height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;

            win_width == mon_width && win_height == mon_height
        }
    }

    fn find_hwnd(&self) -> Option<windows_sys::Win32::Foundation::HWND> {
        find_hwnd_by_title(&self.title)
    }
}

pub fn restore_foreground_by_title(title: &str) -> bool {
    let title = encode_wide(title);
    let Some(hwnd) = find_hwnd_by_title(&title) else {
        return false;
    };
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd) != 0 }
}

pub fn find_hwnd_by_title(title: &[u16]) -> Option<windows_sys::Win32::Foundation::HWND> {
    let hwnd = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(std::ptr::null(), title.as_ptr())
    };
    (!hwnd.is_null()).then_some(hwnd)
}

const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;

#[link(name = "dwmapi")]
extern "system" {
    fn DwmGetWindowAttribute(
        hwnd: windows_sys::Win32::Foundation::HWND,
        dwAttribute: u32,
        pvAttribute: *mut std::ffi::c_void,
        cbAttribute: u32,
    ) -> i32;
}

fn client_rect_for_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> Option<WindowRect> {
    let mut dwm_rect = windows_sys::Win32::Foundation::RECT::default();
    let mut client_rect = windows_sys::Win32::Foundation::RECT::default();
    let mut win_rect = windows_sys::Win32::Foundation::RECT::default();
    let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };

    unsafe {
        let dwm_ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut dwm_rect as *mut windows_sys::Win32::Foundation::RECT).cast(),
            std::mem::size_of::<windows_sys::Win32::Foundation::RECT>() as u32,
        ) == 0;

        let client_ok =
            windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect) != 0;
        let win_ok =
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut win_rect) != 0;
        let pt_ok = windows_sys::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut point) != 0;

        if !client_ok || !pt_ok {
            return None;
        }

        let out = if dwm_ok && win_ok && (win_rect.right - win_rect.left) > 0 {
            let scale_x =
                (dwm_rect.right - dwm_rect.left) as f32 / (win_rect.right - win_rect.left) as f32;
            let scale_y =
                (dwm_rect.bottom - dwm_rect.top) as f32 / (win_rect.bottom - win_rect.top) as f32;

            let border_x = (point.x - win_rect.left) as f32 * scale_x;
            let border_y = (point.y - win_rect.top) as f32 * scale_y;

            WindowRect {
                left: dwm_rect.left + border_x as i32,
                top: dwm_rect.top + border_y as i32,
                width: ((client_rect.right - client_rect.left) as f32 * scale_x) as i32,
                height: ((client_rect.bottom - client_rect.top) as f32 * scale_y) as i32,
            }
        } else {
            WindowRect {
                left: point.x,
                top: point.y,
                width: client_rect.right - client_rect.left,
                height: client_rect.bottom - client_rect.top,
            }
        };

        (out.is_valid()).then_some(out)
    }
}

pub fn encode_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
