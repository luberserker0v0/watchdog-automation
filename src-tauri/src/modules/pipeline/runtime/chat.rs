//! Session-only, structured trace of Patrol's exchange with the VLM.
use base64::Engine as _;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static IMAGES: OnceLock<Mutex<ImageCache>> = OnceLock::new();

#[derive(Default)]
struct ImageCache {
    images: HashMap<u64, (u64, Vec<u8>)>,
    by_hash: HashMap<u64, u64>,
    order: VecDeque<u64>,
    bytes: usize,
}

pub fn cache_image(bytes: Vec<u8>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let digest = hasher.finish();
    if let Ok(mut cache) = IMAGES.get_or_init(Default::default).lock() {
        if let Some(existing_id) = cache.by_hash.get(&digest).copied() {
            if cache
                .images
                .get(&existing_id)
                .is_some_and(|(_, existing)| *existing == bytes)
            {
                return existing_id;
            }
        }
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if bytes.len() > MAX_IMAGE_BYTES {
            return id;
        }
        while cache.bytes + bytes.len() > MAX_IMAGE_BYTES {
            let Some(oldest) = cache.order.pop_front() else {
                break;
            };
            if let Some((hash, old)) = cache.images.remove(&oldest) {
                cache.bytes -= old.len();
                if cache.by_hash.get(&hash) == Some(&oldest) {
                    cache.by_hash.remove(&hash);
                }
            }
        }
        cache.bytes += bytes.len();
        cache.order.push_back(id);
        cache.images.insert(id, (digest, bytes));
        cache.by_hash.insert(digest, id);
        return id;
    }
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[tauri::command]
pub fn get_patrol_chat_image(image_id: u64) -> Result<String, String> {
    let cache = IMAGES
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Patrol image cache unavailable")?;
    let bytes = cache
        .images
        .get(&image_id)
        .ok_or("Image preview is no longer available in this session")?
        .1
        .clone();
    drop(cache);
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

#[tauri::command]
pub fn clear_patrol_chat_images() {
    if let Ok(mut cache) = IMAGES.get_or_init(Default::default).lock() {
        *cache = ImageCache::default();
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContent {
    Text { label: String, text: String },
    Image { label: String, image_id: u64 },
    Score { node_id: String, score: f64 },
    Field { label: String, value: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatrolChatKind {
    Trigger,
    Capture,
    Search,
    Reference,
    Request,
    Response,
    Expansion,
    Report,
    ReferenceSave,
    Error,
    ActionEvidence,
    ActionRequest,
    ActionResponse,
    ActionReport,
    LocationHandoff,
    LocationUnresolved,
    FormatRetry,
    TransportRetry,
    ValidationError,
    RecoveryTrigger,
    RecoveryCandidates,
    RecoveryRequest,
    RecoveryResponse,
    RecoveryReport,
}

impl PatrolChatKind {
    #[cfg(test)]
    const ALL: [Self; 24] = [
        Self::Trigger,
        Self::Capture,
        Self::Search,
        Self::Reference,
        Self::Request,
        Self::Response,
        Self::Expansion,
        Self::Report,
        Self::ReferenceSave,
        Self::Error,
        Self::ActionEvidence,
        Self::ActionRequest,
        Self::ActionResponse,
        Self::ActionReport,
        Self::LocationHandoff,
        Self::LocationUnresolved,
        Self::FormatRetry,
        Self::TransportRetry,
        Self::ValidationError,
        Self::RecoveryTrigger,
        Self::RecoveryCandidates,
        Self::RecoveryRequest,
        Self::RecoveryResponse,
        Self::RecoveryReport,
    ];
}

#[derive(Clone, serde::Serialize)]
struct ChatEvent {
    id: u64,
    run_id: String,
    sample_id: String,
    occurred_at_ms: u128,
    sender: &'static str,
    kind: PatrolChatKind,
    summary: String,
    node_id: String,
    target: String,
    session: u64,
    frame: u64,
    tier: Option<&'static str>,
    content: Vec<ChatContent>,
}

#[derive(Clone)]
pub struct PatrolChatSink {
    app: AppHandle,
    run_id: String,
    node_id: String,
    target: String,
    session: u64,
    frame: u64,
}

impl PatrolChatSink {
    pub fn new(
        app: &AppHandle,
        run_id: &str,
        node_id: &str,
        target: &str,
        session: u64,
        frame: u64,
    ) -> Self {
        Self {
            app: app.clone(),
            run_id: run_id.into(),
            node_id: node_id.into(),
            target: target.into(),
            session,
            frame,
        }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn emit(
        &self,
        sender: &'static str,
        kind: PatrolChatKind,
        summary: impl Into<String>,
        tier: Option<&'static str>,
        content: Vec<ChatContent>,
    ) {
        let event = ChatEvent {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            run_id: self.run_id.clone(),
            sample_id: format!("{}:{}:{}", self.run_id, self.session, self.frame),
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_millis()),
            sender,
            kind,
            summary: summary.into(),
            node_id: self.node_id.clone(),
            target: self.target.clone(),
            session: self.session,
            frame: self.frame,
            tier,
            content,
        };
        let _ = self.app.emit("patrol-chat-event", event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_are_fetched_by_id_and_disappear_after_clear() {
        let image_id = cache_image(vec![1, 2, 3]);
        assert_eq!(cache_image(vec![1, 2, 3]), image_id);
        assert_eq!(
            get_patrol_chat_image(image_id).unwrap(),
            "data:image/jpeg;base64,AQID"
        );
        clear_patrol_chat_images();
        assert!(get_patrol_chat_image(image_id).is_err());
    }

    #[test]
    fn trace_event_contains_only_an_image_id() {
        let event = ChatEvent {
            id: 1,
            run_id: "run".into(),
            sample_id: "run:1:2".into(),
            occurred_at_ms: 0,
            sender: "patrol",
            kind: PatrolChatKind::Capture,
            summary: "Captured".into(),
            node_id: "A".into(),
            target: "Screen 0".into(),
            session: 1,
            frame: 2,
            tier: None,
            content: vec![ChatContent::Image {
                label: "Capture".into(),
                image_id: 99,
            }],
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["content"][0]["image_id"], 99);
        assert!(value.to_string().find("base64").is_none());
        assert!(value.to_string().find("api_key").is_none());
    }

    #[test]
    fn every_chat_kind_has_a_stable_snake_case_wire_value() {
        let values = PatrolChatKind::ALL
            .into_iter()
            .map(|kind| serde_json::to_value(kind).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 24);
        assert!(values.contains(&serde_json::json!("action_evidence")));
        assert!(values.contains(&serde_json::json!("recovery_candidates")));
        assert!(values.contains(&serde_json::json!("location_handoff")));
        assert!(values.contains(&serde_json::json!("location_unresolved")));
        assert!(values.contains(&serde_json::json!("error")));
    }
}
