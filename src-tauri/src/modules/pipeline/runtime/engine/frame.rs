//! Fresh-frame acquisition for a running pipeline session.
use super::{PipelineState, RunHandle};
use crate::modules::capture::services::stream::{StreamFrame, StreamService, StreamTarget};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

pub(crate) async fn receive_after(
    stream_service: &StreamService,
    rx: &mut broadcast::Receiver<StreamFrame>,
    not_before: Option<Instant>,
    min_sequence: Option<u64>,
    timeout: Duration,
    expected_session: Option<u64>,
    expected_target: Option<&StreamTarget>,
    handle: Option<&RunHandle>,
) -> Result<(StreamFrame, usize, Duration), (String, Duration)> {
    let mut stale_frames_skipped = 0;
    let mut active_elapsed = Duration::ZERO;
    let mut paused_elapsed = Duration::ZERO;
    if let (Some(session), Some(target)) = (expected_session, expected_target) {
        if let Some(frame) =
            stream_service.latest_frame_after(session, target, min_sequence, not_before)
        {
            return Ok((frame, 0, paused_elapsed));
        }
    }
    while active_elapsed < timeout {
        if let Some(handle) = handle {
            if handle
                .should_stop
                .lock()
                .map(|value| *value)
                .unwrap_or(true)
            {
                return Err((
                    "pipeline stopped while waiting for a frame".to_string(),
                    paused_elapsed,
                ));
            }
            if matches!(handle.get_state(), PipelineState::Paused) {
                let pause_started = Instant::now();
                tokio::time::sleep(Duration::from_millis(25)).await;
                paused_elapsed += pause_started.elapsed();
                continue;
            }
        }
        let slice = (timeout - active_elapsed).min(Duration::from_millis(50));
        let started = Instant::now();
        match tokio::time::timeout(slice, rx.recv()).await {
            Ok(Ok(frame))
                if not_before.map_or(true, |boundary| frame.captured_at >= boundary)
                    && expected_session.map_or(true, |session| frame.session_id == session)
                    && expected_target.map_or(true, |target| &frame.target == target) =>
            {
                if min_sequence.is_some_and(|sequence| frame.sequence <= sequence) {
                    stale_frames_skipped += 1;
                } else {
                    return Ok((frame, stale_frames_skipped, paused_elapsed));
                }
            }
            Ok(Ok(_)) => stale_frames_skipped += 1,
            Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                stale_frames_skipped += skipped as usize
            }
            Ok(Err(error)) => {
                return Err((format!("stream receive failed: {error}"), paused_elapsed))
            }
            Err(_) => {}
        }
        active_elapsed += started.elapsed();
    }
    Err(("fresh stream frame timed out".to_string(), paused_elapsed))
}
