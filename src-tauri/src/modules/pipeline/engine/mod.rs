pub mod debug_image;
pub mod runner;
pub mod watcher;

use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::models::recognizer::Region;
use crate::modules::pipeline::plugins::PluginRegistry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineState {
    Idle,
    Running,
    Paused,
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
}

pub struct ExecutionContext {
    pub current_node_id: String,
    pub graph: PipelineGraph,
    pub registry: PluginRegistry,
    pub state: PipelineState,
    pub step_results: Vec<StepResult>,
    pub recognized_regions: HashMap<String, Region>,
    /// Capture-local recognition boxes used only for debug-image overlays.
    pub debug_recognized_regions: HashMap<String, Region>,
    pub hit_counts: HashMap<String, u32>,
    pub user_params: HashMap<String, serde_json::Value>,
    pub start_time: std::time::Instant,
    pub failed: bool,
    pub save_debug_images: bool,
    pub hwnd: Option<usize>,
}

impl ExecutionContext {
    pub fn new(
        graph: PipelineGraph,
        registry: PluginRegistry,
        user_params: HashMap<String, serde_json::Value>,
    ) -> Self {
        let entry = graph.entry_node().map(|n| n.id.clone()).unwrap_or_default();

        Self {
            current_node_id: entry,
            graph,
            registry,
            state: PipelineState::Idle,
            step_results: Vec::new(),
            recognized_regions: HashMap::new(),
            debug_recognized_regions: HashMap::new(),
            hit_counts: HashMap::new(),
            user_params,
            start_time: std::time::Instant::now(),
            failed: false,
            save_debug_images: false,
            hwnd: None,
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
        }
    }

    pub fn stop(&self) {
        if let Ok(mut stop) = self.should_stop.lock() {
            *stop = true;
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

        PipelineStatus {
            run_id: self.run_id.clone(),
            state: match &state {
                PipelineState::Idle => "idle",
                PipelineState::Running => "running",
                PipelineState::Paused => "paused",
                PipelineState::Completed => "completed",
                PipelineState::Failed(_) => "failed",
            }
            .to_string(),
            current_node_id,
            step_count,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            step_results,
            original_desktop,
        }
    }
}
