use crate::core::error::CaptureError;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_capture::d3d11::StagingTexture;
use windows_capture::dxgi_duplication_api::{DxgiDuplicationApi, Error as DxgiError};
use windows_capture::monitor::Monitor;

pub struct DxgiCapture {
    monitor: Monitor,
    dup_api: Option<DxgiDuplicationApi>,
    staging: Option<StagingTexture>,
    staging_width: u32,
    staging_height: u32,
    compressor: Option<turbojpeg::Compressor>,
    last_frame: Option<Vec<u8>>,
    raw_buffer: Vec<u8>,
    crop_buffer: Vec<u8>,
}

impl DxgiCapture {
    pub fn new(monitor: Monitor) -> Self {
        Self {
            monitor,
            dup_api: None,
            staging: None,
            staging_width: 0,
            staging_height: 0,
            compressor: None,
            last_frame: None,
            raw_buffer: Vec::with_capacity(1920 * 1080 * 4),
            crop_buffer: Vec::with_capacity(1920 * 1080 * 4),
        }
    }

    fn ensure_dup_api(&mut self) -> Result<(), CaptureError> {
        if self.dup_api.is_none() {
            let dup = DxgiDuplicationApi::new(self.monitor)
                .map_err(|e| CaptureError::CaptureFailed(format!("DXGI init error: {}", e)))?;
            self.dup_api = Some(dup);
            self.staging = None;
            self.staging_width = 0;
            self.staging_height = 0;
        }
        Ok(())
    }

    fn ensure_staging(&mut self, width: u32, height: u32) -> Result<(), CaptureError> {
        if self.staging.is_some() && self.staging_width == width && self.staging_height == height {
            return Ok(());
        }
        let device = self.dup_api.as_ref().unwrap().device();
        let staging = StagingTexture::new(device, width, height, DXGI_FORMAT_B8G8R8A8_UNORM)
            .map_err(|e| {
                CaptureError::CaptureFailed(format!("DXGI staging texture error: {}", e))
            })?;
        self.staging = Some(staging);
        self.staging_width = width;
        self.staging_height = height;
        Ok(())
    }

    fn ensure_compressor(&mut self) -> Result<(), CaptureError> {
        if self.compressor.is_none() {
            let mut compressor = turbojpeg::Compressor::new().map_err(|e| {
                CaptureError::CaptureFailed(format!("JPEG compressor error: {}", e))
            })?;
            let _ = compressor.set_quality(85);
            let _ = compressor.set_subsamp(turbojpeg::Subsamp::None);
            self.compressor = Some(compressor);
        }
        Ok(())
    }

    pub fn capture_jpeg(
        &mut self,
        timeout_ms: u32,
        crop: Option<(usize, usize, usize, usize)>,
    ) -> Result<Vec<u8>, CaptureError> {
        self.ensure_dup_api()?;
        self.ensure_compressor()?;

        let width = self.dup_api.as_ref().unwrap().width();
        let height = self.dup_api.as_ref().unwrap().height();
        self.ensure_staging(width, height)?;

        let dup = self.dup_api.as_mut().unwrap();
        let staging = self.staging.as_mut().unwrap();
        let compressor = self.compressor.as_mut().unwrap();
        let raw_buffer = &mut self.raw_buffer;
        let crop_buffer = &mut self.crop_buffer;
        let last_frame = &mut self.last_frame;

        capture_frame(
            dup,
            staging,
            compressor,
            raw_buffer,
            crop_buffer,
            last_frame,
            timeout_ms,
            self.monitor,
            crop,
        )
    }
}

fn capture_frame(
    dup: &mut DxgiDuplicationApi,
    staging: &mut StagingTexture,
    compressor: &mut turbojpeg::Compressor,
    raw_buffer: &mut Vec<u8>,
    crop_buffer: &mut Vec<u8>,
    last_frame: &mut Option<Vec<u8>>,
    timeout_ms: u32,
    monitor: Monitor,
    crop: Option<(usize, usize, usize, usize)>,
) -> Result<Vec<u8>, CaptureError> {
    let mut frame = match dup.acquire_next_frame(timeout_ms) {
        Ok(f) => f,
        Err(DxgiError::AccessLost) => {
            log::debug!("DXGI access lost, recreating duplication");
            *dup = recreate_dup(monitor)?;
            dup.acquire_next_frame(timeout_ms).map_err(|e| match e {
                DxgiError::Timeout => {
                    CaptureError::CaptureFailed("DXGI timeout after recreate".to_string())
                }
                _ => CaptureError::CaptureFailed(format!("DXGI error: {}", e)),
            })?
        }
        Err(DxgiError::Timeout) => {
            return match last_frame {
                Some(frame) => {
                    log::debug!("DXGI timeout, returning cached frame");
                    Ok(frame.clone())
                }
                None => Err(CaptureError::CaptureFailed(
                    "DXGI timeout, no cached frame".to_string(),
                )),
            };
        }
        Err(e) => return Err(CaptureError::CaptureFailed(format!("DXGI error: {}", e))),
    };

    let w = frame.width() as usize;
    let h = frame.height() as usize;

    raw_buffer.clear();
    let frame_buffer = frame
        .buffer_with(staging)
        .map_err(|e| CaptureError::CaptureFailed(format!("DXGI buffer error: {}", e)))?;
    let bgra = frame_buffer.as_nopadding_buffer(raw_buffer);

    let (crop_x, crop_y, crop_width, crop_height) = crop.unwrap_or((0, 0, w, h));
    let crop_x = crop_x.min(w.saturating_sub(1));
    let crop_y = crop_y.min(h.saturating_sub(1));
    let crop_width = crop_width.min(w - crop_x).max(1);
    let crop_height = crop_height.min(h - crop_y).max(1);
    let is_full_frame = crop_x == 0 && crop_y == 0 && crop_width == w && crop_height == h;
    let (pixels, pitch) = if is_full_frame {
        (bgra, w * 4)
    } else {
        crop_buffer.clear();
        crop_buffer.reserve(crop_width * crop_height * 4);
        for row in crop_y..crop_y + crop_height {
            let start = (row * w + crop_x) * 4;
            crop_buffer.extend_from_slice(&bgra[start..start + crop_width * 4]);
        }
        (&crop_buffer[..], crop_width * 4)
    };
    let image = turbojpeg::Image {
        pixels,
        width: crop_width,
        pitch,
        height: crop_height,
        format: turbojpeg::PixelFormat::BGRA,
    };

    let jpeg = compressor
        .compress_to_vec(image)
        .map_err(|e| CaptureError::CaptureFailed(format!("JPEG encode error: {}", e)))?;
    let jpeg_bytes = jpeg.to_vec();

    *last_frame = Some(jpeg_bytes.clone());
    Ok(jpeg_bytes)
}

fn recreate_dup(monitor: Monitor) -> Result<DxgiDuplicationApi, CaptureError> {
    DxgiDuplicationApi::new(monitor)
        .map_err(|e| CaptureError::CaptureFailed(format!("DXGI init error: {}", e)))
}
