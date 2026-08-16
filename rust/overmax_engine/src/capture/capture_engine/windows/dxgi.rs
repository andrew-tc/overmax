use crate::capture::capture_engine::CaptureEngine;
use crate::capture::frame::CapturedFrame;
use crate::capture::window_tracker::WindowRect;

use windows::core::Interface;
use windows::Win32::Foundation::{HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIAdapter, IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, DXGI_OUTDUPL_FRAME_INFO,
};

pub struct DxgiCaptureEngine {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    adapter: IDXGIAdapter,
    duplication: IDXGIOutputDuplication,
    staging_texture: Option<ID3D11Texture2D>,
    width: u32,
    height: u32,
    output_bounds: RECT,
}

unsafe impl Send for DxgiCaptureEngine {}
unsafe impl Sync for DxgiCaptureEngine {}

impl DxgiCaptureEngine {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let mut device = None;
            let mut context = None;
            let mut level = D3D_FEATURE_LEVEL_11_0;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut level),
                Some(&mut context),
            )
            .map_err(|e| format!("D3D11CreateDevice failed: {e}"))?;

            let device = device.ok_or("D3D11 device not created")?;
            let context = context.ok_or("D3D11 context not created")?;

            let dxgi_device: IDXGIDevice = device
                .cast()
                .map_err(|e| format!("Query IDXGIDevice failed: {e}"))?;
            let adapter = dxgi_device
                .GetAdapter()
                .map_err(|e| format!("GetAdapter failed: {e}"))?;

            let (duplication, width, height, output_bounds) =
                Self::find_output(&adapter, &device, None)?;

            Ok(Self {
                device,
                context,
                adapter,
                duplication,
                staging_texture: None,
                width,
                height,
                output_bounds,
            })
        }
    }

    fn find_output(
        adapter: &IDXGIAdapter,
        device: &ID3D11Device,
        rect_opt: Option<WindowRect>,
    ) -> Result<(IDXGIOutputDuplication, u32, u32, RECT), String> {
        unsafe {
            let mut best_output = None;
            let mut first_output = None;

            let mut i = 0;
            while let Ok(output) = adapter.EnumOutputs(i) {
                if let Ok(desc) = output.GetDesc() {
                    let bounds = desc.DesktopCoordinates;
                    if first_output.is_none() {
                        first_output = Some((output.clone(), bounds));
                    }
                    if let Some(rect) = rect_opt {
                        let center_x = rect.left + rect.width / 2;
                        let center_y = rect.top + rect.height / 2;
                        if center_x >= bounds.left
                            && center_x < bounds.right
                            && center_y >= bounds.top
                            && center_y < bounds.bottom
                        {
                            best_output = Some((output, bounds));
                            break;
                        }
                    }
                }
                i += 1;
            }

            let (output, bounds) = best_output.or(first_output).ok_or("No DXGI output found")?;

            let output1: IDXGIOutput1 = output
                .cast()
                .map_err(|e| format!("Query IDXGIOutput1 failed: {e}"))?;

            let duplication = output1
                .DuplicateOutput(device)
                .map_err(|e| format!("DuplicateOutput failed: {e}"))?;
            let desc = duplication.GetDesc();

            Ok((
                duplication,
                desc.ModeDesc.Width,
                desc.ModeDesc.Height,
                bounds,
            ))
        }
    }

    fn ensure_output_for_rect(&mut self, rect: WindowRect) -> Result<(), String> {
        let center_x = rect.left + rect.width / 2;
        let center_y = rect.top + rect.height / 2;
        let inside = center_x >= self.output_bounds.left
            && center_x < self.output_bounds.right
            && center_y >= self.output_bounds.top
            && center_y < self.output_bounds.bottom;

        if !inside {
            if let Ok((duplication, width, height, bounds)) =
                Self::find_output(&self.adapter, &self.device, Some(rect))
            {
                self.duplication = duplication;
                self.width = width;
                self.height = height;
                self.output_bounds = bounds;
                self.staging_texture = None;
            }
        }
        Ok(())
    }

    fn ensure_staging_texture(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.staging_texture.is_none() {
            unsafe {
                let desc = D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    MiscFlags: 0,
                };
                let mut texture = None;
                self.device
                    .CreateTexture2D(&desc, None, Some(&mut texture))
                    .map_err(|e| format!("Create staging texture failed: {e}"))?;
                self.staging_texture = Some(texture.ok_or("Staging texture is None")?);
            }
        }
        Ok(())
    }
}

impl CaptureEngine for DxgiCaptureEngine {
    fn capture_bgra(&mut self, rect: WindowRect) -> Result<CapturedFrame, String> {
        let mut frame = CapturedFrame {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        };
        self.capture_bgra_inplace(rect, &mut frame)?;
        Ok(frame)
    }

    fn capture_bgra_inplace(
        &mut self,
        rect: WindowRect,
        out_frame: &mut CapturedFrame,
    ) -> Result<(), String> {
        if !rect.is_valid() {
            return Err("Capture rect must have positive dimensions".to_string());
        }

        let _ = self.ensure_output_for_rect(rect);

        unsafe {
            self.ensure_staging_texture(self.width, self.height)?;

            let mut resource = None;
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();

            let acquire_res = self
                .duplication
                .AcquireNextFrame(0, &mut frame_info, &mut resource);

            let staging = self
                .staging_texture
                .as_ref()
                .ok_or("Staging texture missing")?;

            match acquire_res {
                Ok(_) => {
                    if let Some(res) = resource {
                        let texture: ID3D11Texture2D = res
                            .cast()
                            .map_err(|e| format!("Query ID3D11Texture2D failed: {e}"))?;
                        self.context.CopyResource(staging, &texture);
                        let _ = self.duplication.ReleaseFrame();

                        return crop_texture_to_buffer(
                            &self.context,
                            staging,
                            self.width,
                            self.height,
                            rect,
                            self.output_bounds,
                            out_frame,
                        );
                    }
                    let _ = self.duplication.ReleaseFrame();
                }
                Err(err) => {
                    let code = err.code().0 as u32;
                    if code == 0x887A0027 {
                        if out_frame.bgra.is_empty() {
                            return Err("DXGI initial frame timeout".to_string());
                        }
                    } else {
                        return Err(format!(
                            "DXGI AcquireNextFrame failed with HRESULT 0x{:X}: {}",
                            code, err
                        ));
                    }
                }
            }

            crop_texture_to_buffer(
                &self.context,
                staging,
                self.width,
                self.height,
                rect,
                self.output_bounds,
                out_frame,
            )
        }
    }
}

unsafe fn crop_texture_to_buffer(
    context: &ID3D11DeviceContext,
    staging: &ID3D11Texture2D,
    desktop_width: u32,
    desktop_height: u32,
    rect: WindowRect,
    output_bounds: RECT,
    out_frame: &mut CapturedFrame,
) -> Result<(), String> {
    let mut mapped = Default::default();
    context
        .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .map_err(|e| format!("Map texture failed: {e}"))?;

    let row_pitch = mapped.RowPitch as usize;
    let data_ptr = mapped.pData as *const u8;

    let local_left = rect.left - output_bounds.left;
    let local_top = rect.top - output_bounds.top;

    let start_x = local_left.clamp(0, desktop_width as i32) as usize;
    let start_y = local_top.clamp(0, desktop_height as i32) as usize;
    let crop_width = (rect.width as usize).min(desktop_width as usize - start_x);
    let crop_height = (rect.height as usize).min(desktop_height as usize - start_y);

    let len = crop_width * crop_height * 4;
    out_frame.width = crop_width as i32;
    out_frame.height = crop_height as i32;
    out_frame.bgra.resize(len, 0);

    for y in 0..crop_height {
        let src_offset = (start_y + y) * row_pitch + start_x * 4;
        let dst_offset = y * crop_width * 4;
        let src_row = data_ptr.add(src_offset);
        let dst_row = out_frame.bgra.as_mut_ptr().add(dst_offset);
        std::ptr::copy_nonoverlapping(src_row, dst_row, crop_width * 4);
    }

    context.Unmap(staging, 0);
    Ok(())
}
