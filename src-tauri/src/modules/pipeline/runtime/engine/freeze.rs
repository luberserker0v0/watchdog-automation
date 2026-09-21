//! Bounded, cancellable visual stability wait on one capture session.
use super::{frame, RunHandle};
use crate::modules::capture::services::stream::{StreamFrame, StreamService, StreamTarget};
use crate::modules::pipeline::authoring::models::node::FreezeWait;
use image::{imageops::FilterType, DynamicImage, GrayImage};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

pub enum FreezeOutcome {
    Stable(StreamFrame),
    Stopped,
    Failed(String),
}

fn sample(image: &DynamicImage, roi: Option<[u32; 4]>) -> Result<GrayImage, String> {
    let reduced = if let Some([x, y, width, height]) = roi {
        if width == 0
            || height == 0
            || x.checked_add(width)
                .is_none_or(|right| right > image.width())
            || y.checked_add(height)
                .is_none_or(|bottom| bottom > image.height())
        {
            return Err("Freeze wait ROI is outside the captured frame".into());
        }
        image
            .crop_imm(x, y, width, height)
            .resize_exact(64, 64, FilterType::Triangle)
    } else {
        image.resize_exact(64, 64, FilterType::Triangle)
    };
    Ok(reduced.to_luma8())
}

fn similarity(left: &GrayImage, right: &GrayImage) -> f64 {
    let difference: u64 = left
        .pixels()
        .zip(right.pixels())
        .map(|(a, b)| a.0[0].abs_diff(b.0[0]) as u64)
        .sum();
    1.0 - difference as f64 / (left.width() as f64 * left.height() as f64 * 255.0)
}

#[allow(clippy::too_many_arguments)]
pub async fn wait(
    service: &StreamService,
    rx: &mut broadcast::Receiver<StreamFrame>,
    config: &FreezeWait,
    session: u64,
    target: &StreamTarget,
    initial: Option<(&DynamicImage, u64)>,
    handle: &RunHandle,
    max_active_wait: Option<Duration>,
) -> FreezeOutcome {
    let (time_ms, threshold, rate_ms, timeout_ms, roi) = match config.settings() {
        Ok(Some(settings)) => settings,
        Ok(None) => return FreezeOutcome::Failed("Disabled freeze wait was invoked".into()),
        Err(error) => return FreezeOutcome::Failed(error),
    };
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms).min(max_active_wait.unwrap_or(Duration::MAX));
    let mut paused = Duration::ZERO;
    let mut previous = match initial {
        Some((image, _)) => match sample(image, roi) {
            Ok(value) => Some(value),
            Err(error) => return FreezeOutcome::Failed(error),
        },
        None => None,
    };
    let mut previous_size = initial.map(|(image, _)| (image.width(), image.height()));
    let mut stable_since_active = initial.map(|_| Duration::ZERO);
    let mut sequence = initial.map(|(_, sequence)| sequence);
    let mut not_before = Some(started);
    loop {
        let active = started.elapsed().saturating_sub(paused);
        if active >= timeout {
            return FreezeOutcome::Failed(format!(
                "Screen did not remain stable for {time_ms} ms within {} ms active wait",
                timeout.as_millis()
            ));
        }
        let slice = (timeout - active).min(Duration::from_millis(rate_ms + 250));
        match frame::receive_after(
            service,
            rx,
            not_before,
            sequence,
            slice,
            Some(session),
            Some(target),
            Some(handle),
        )
        .await
        {
            Ok((frame, _, paused_time)) => {
                paused += paused_time;
                let image = match image::load_from_memory(&frame.bytes) {
                    Ok(image) => image,
                    Err(error) => {
                        return FreezeOutcome::Failed(format!(
                            "Cannot decode freeze frame: {error}"
                        ))
                    }
                };
                let current = match sample(&image, roi) {
                    Ok(value) => value,
                    Err(error) => return FreezeOutcome::Failed(error),
                };
                let active_now = started.elapsed().saturating_sub(paused);
                if previous_size == Some((image.width(), image.height()))
                    && previous
                        .as_ref()
                        .is_some_and(|prev| similarity(prev, &current) >= threshold)
                {
                    if stable_since_active.is_some_and(|since| {
                        active_now.saturating_sub(since) >= Duration::from_millis(time_ms)
                    }) {
                        return FreezeOutcome::Stable(frame);
                    }
                } else {
                    stable_since_active = Some(active_now);
                }
                previous = Some(current);
                previous_size = Some((image.width(), image.height()));
                sequence = Some(frame.sequence);
                not_before = Some(frame.captured_at + Duration::from_millis(rate_ms));
            }
            Err((error, paused_time)) => {
                paused += paused_time;
                if error.contains("pipeline stopped") {
                    return FreezeOutcome::Stopped;
                }
                if error != "fresh stream frame timed out" {
                    return FreezeOutcome::Failed(error);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_frames_score_high_and_changes_score_low() {
        let a = sample(&DynamicImage::new_rgb8(100, 100), None).unwrap();
        let b = sample(&DynamicImage::new_rgb8(100, 100), None).unwrap();
        let white = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            100,
            100,
            image::Rgb([255, 255, 255]),
        ));
        let c = sample(&white, None).unwrap();
        assert_eq!(similarity(&a, &b), 1.0);
        assert!(similarity(&a, &c) < 0.1);
        assert!(sample(&white, Some([90, 90, 20, 20])).is_err());
    }
}
