pub mod debug_image;
pub mod frame;
pub mod runner;
pub mod routing;
pub mod step;
pub mod watcher;

use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::models::recognizer::Region;
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::modules::pipeline::plan::ExecutionPlan;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineState {
    Idle,
    Running,
    Paused,
    Stopping,
    Stopped,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineStatus {
    pub run_id: String,
    pub state: String,
    pub current_node_id: Option<String>,
    pub step_count: usize,
    pub elapsed_ms: u64,
    pub step_results: Vec<StepResult>,
    pub original_desktop: Option<String>,
    pub error: Option<String>,
    pub attempt_count: usize,
    pub effective_target: Option<crate::modules::pipeline::models::graph::PipelineTarget>,
    #[serde(default)]
    pub hit_counts: HashMap<String, u32>,
    #[serde(default)]
    pub children_parent_id: Option<String>,
    #[serde(default)]
    pub children_start_time_ms: Option<u64>,
    #[serde(default)]
    pub children_child_id: Option<String>,
    #[serde(default)]
    pub children_timeout_ms: Option<u64>,
    /// Monotonic per-run revision used by event consumers to ignore stale UI updates.
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineStatusEvent {
    pub run_id: String,
    pub revision: u64,
    pub state: String,
    pub current_node_id: Option<String>,
    pub step_count: usize,
    pub elapsed_ms: u64,
    pub error: Option<String>,
    pub attempt_count: usize,
    pub effective_target: Option<crate::modules::pipeline::models::graph::PipelineTarget>,
    pub hit_counts: HashMap<String, u32>,
    pub children_parent_id: Option<String>,
    pub children_start_time_ms: Option<u64>,
    pub children_child_id: Option<String>,
    pub children_timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appended_step: Option<StepResult>,
}

/// Children-trying state for a parent node. Every node with `n >= 1`
/// outgoing children runs under this state: candidates are tried
/// round-robin against the parent's timeout quota (`timeout` for normal
/// children, `error_timeout` for the error target).
pub struct ChildrenState {
    pub parent_node_id: String,
    pub candidates: Vec<String>,
    pub current_index: usize,
    pub start_time: std::time::Instant,
    pub timeout_ms: u64,
    pub kind: ChildrenKind,
}

/// Re-exported so engine submodules share one definition.
pub use routing::ChildrenKind;

pub struct ExecutionContext {
    pub current_node_id: String,
    /// Immutable compiled routes. The graph remains available for existing
    /// recognizer/action code, while all transitions use this plan.
    pub plan: ExecutionPlan,
    pub graph: PipelineGraph,
    pub registry: PluginRegistry,
    pub state: PipelineState,
    pub step_results: Vec<StepResult>,
    pub recognized_regions: HashMap<String, Region>,
    /// Target identity of each successfully recognized box.
    pub recognized_region_targets: HashMap<String, crate::services::stream::StreamTarget>,
    /// Capture-local recognition boxes used only for debug-image overlays.
    pub debug_recognized_regions: HashMap<String, Region>,
    pub hit_counts: HashMap<String, u32>,
    pub user_params: HashMap<String, HashMap<String, serde_json::Value>>,
    pub start_time: std::time::Instant,
    pub failed: bool,
    pub failure_error: Option<String>,
    pub save_debug_images: bool,
    pub hwnd: Option<usize>,
    pub children: Option<ChildrenState>,
}

impl ExecutionContext {
    pub fn new(
        plan: ExecutionPlan,
        registry: PluginRegistry,
        user_params: HashMap<String, HashMap<String, serde_json::Value>>,
    ) -> Self {
        let entry = plan.entry_node_id().to_string();
        let graph = plan.graph().clone();

        Self {
            current_node_id: entry,
            plan,
            graph,
            registry,
            state: PipelineState::Idle,
            step_results: Vec::new(),
            recognized_regions: HashMap::new(),
            recognized_region_targets: HashMap::new(),
            debug_recognized_regions: HashMap::new(),
            hit_counts: HashMap::new(),
            user_params,
            start_time: std::time::Instant::now(),
            failed: false,
            failure_error: None,
            save_debug_images: false,
            hwnd: None,
            children: None,
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StepResult {
    pub node_id: String,
    pub matched: bool,
    pub confidence: f64,
    pub box_region: Option<Region>,
    pub actions_fired: usize,
    #[serde(default)]
    pub skipped: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timings_ms: Option<std::collections::BTreeMap<String, u64>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineResult {
    pub success: bool,
    pub steps: Vec<StepResult>,
    pub error: Option<String>,
}

pub struct RunHandle {
    pub run_id: String,
    pub state: Arc<Mutex<PipelineState>>,
    pub should_stop: Arc<Mutex<bool>>,
    pub current_node_id: Arc<Mutex<Option<String>>>,
    pub step_count: Arc<Mutex<usize>>,
    pub step_results: Arc<Mutex<Vec<StepResult>>>,
    pub start_time: std::time::Instant,
    pub original_desktop: Arc<Mutex<Option<String>>>,
    pub attempt_count: Arc<Mutex<usize>>,
    pub effective_target:
        Arc<Mutex<Option<crate::modules::pipeline::models::graph::PipelineTarget>>>,
    pub hit_counts: Arc<Mutex<HashMap<String, u32>>>,
    pub children_parent_id: Arc<Mutex<Option<String>>>,
    pub children_start_time: Arc<Mutex<Option<std::time::Instant>>>,
    pub children_child_id: Arc<Mutex<Option<String>>>,
    pub children_timeout_ms: Arc<Mutex<Option<u64>>>,
    revision: AtomicU64,
}

impl RunHandle {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            state: Arc::new(Mutex::new(PipelineState::Idle)),
            should_stop: Arc::new(Mutex::new(false)),
            current_node_id: Arc::new(Mutex::new(None)),
            step_count: Arc::new(Mutex::new(0)),
            step_results: Arc::new(Mutex::new(Vec::new())),
            start_time: std::time::Instant::now(),
            original_desktop: Arc::new(Mutex::new(None)),
            attempt_count: Arc::new(Mutex::new(0)),
            effective_target: Arc::new(Mutex::new(None)),
            hit_counts: Arc::new(Mutex::new(HashMap::new())),
            children_parent_id: Arc::new(Mutex::new(None)),
            children_start_time: Arc::new(Mutex::new(None)),
            children_child_id: Arc::new(Mutex::new(None)),
            children_timeout_ms: Arc::new(Mutex::new(None)),
            revision: AtomicU64::new(0),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut stop) = self.should_stop.lock() {
            *stop = true;
        }
        if let Ok(mut state) = self.state.lock() {
            if matches!(*state, PipelineState::Running | PipelineState::Paused) {
                *state = PipelineState::Stopping;
            }
        }
    }

    pub fn get_state(&self) -> PipelineState {
        self.state
            .lock()
            .map(|s| s.clone())
            .unwrap_or(PipelineState::Failed("Lock poisoned".to_string()))
    }

    pub fn status(&self) -> PipelineStatus {
        let current_node_id = self.current_node_id.lock().ok().and_then(|s| s.clone());
        let step_count = self.step_count.lock().map(|s| *s).unwrap_or(0);
        let step_results = self
            .step_results
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let state = self.get_state();
        let original_desktop = self.original_desktop.lock().ok().and_then(|s| s.clone());
        let attempt_count = self.attempt_count.lock().map(|value| *value).unwrap_or(0);
        let effective_target = self
            .effective_target
            .lock()
            .ok()
            .and_then(|value| value.clone());
        let hit_counts = self
            .hit_counts
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let children_parent_id = self.children_parent_id.lock().ok().and_then(|s| s.clone());
        let children_child_id = self.children_child_id.lock().ok().and_then(|s| s.clone());
        let children_timeout_ms = self.children_timeout_ms.lock().ok().and_then(|v| *v);
        let children_start_time_ms = self
            .children_start_time
            .lock()
            .ok()
            .and_then(|t| t.map(|start| start.elapsed().as_millis() as u64));

        PipelineStatus {
            run_id: self.run_id.clone(),
            state: match &state {
                PipelineState::Idle => "idle",
                PipelineState::Running => "running",
                PipelineState::Paused => "paused",
                PipelineState::Stopping => "stopping",
                PipelineState::Stopped => "stopped",
                PipelineState::Completed => "completed",
                PipelineState::Failed(_) => "failed",
            }
            .to_string(),
            current_node_id,
            step_count,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            step_results,
            original_desktop,
            error: match state {
                PipelineState::Failed(message) => Some(message),
                _ => None,
            },
            attempt_count,
            effective_target,
            hit_counts,
            children_parent_id,
            children_start_time_ms,
            children_child_id,
            children_timeout_ms,
            revision: self.revision.load(Ordering::Acquire),
        }
    }

    /// Advance the run revision only after a visible state transition. This
    /// keeps event delivery ordered without requiring the frontend to infer
    /// ordering from wall-clock time.
    pub fn next_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn event(&self, appended_step: Option<StepResult>) -> PipelineStatusEvent {
        let revision = self.next_revision();
        let state = self.get_state();
        let children_start_time_ms = self.children_start_time.lock().ok()
            .and_then(|value| value.map(|started| started.elapsed().as_millis() as u64));
        PipelineStatusEvent {
            run_id: self.run_id.clone(),
            revision,
            state: match &state {
                PipelineState::Idle => "idle", PipelineState::Running => "running",
                PipelineState::Paused => "paused", PipelineState::Stopping => "stopping",
                PipelineState::Stopped => "stopped", PipelineState::Completed => "completed",
                PipelineState::Failed(_) => "failed",
            }.to_string(),
            current_node_id: self.current_node_id.lock().ok().and_then(|value| value.clone()),
            step_count: self.step_count.lock().map(|value| *value).unwrap_or(0),
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            error: match state { PipelineState::Failed(message) => Some(message), _ => None },
            attempt_count: self.attempt_count.lock().map(|value| *value).unwrap_or(0),
            effective_target: self.effective_target.lock().ok().and_then(|value| value.clone()),
            hit_counts: self.hit_counts.lock().map(|value| value.clone()).unwrap_or_default(),
            children_parent_id: self.children_parent_id.lock().ok().and_then(|value| value.clone()),
            children_child_id: self.children_child_id.lock().ok().and_then(|value| value.clone()),
            children_timeout_ms: self.children_timeout_ms.lock().ok().and_then(|value| *value),
            children_start_time_ms,
            appended_step,
        }
    }
}
