use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::parser;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    pub pending_graph: Arc<Mutex<Option<PipelineGraph>>>,
}

impl FileWatcher {
    pub fn new(mermaid_path: PathBuf, meta_path: PathBuf) -> Result<Self, String> {
        let pending_graph = Arc::new(Mutex::new(None));
        let pending_graph_clone = pending_graph.clone();

        let mermaid_path_clone = mermaid_path.clone();
        let meta_path_clone = meta_path.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        let _ = reload_pipeline(
                            &mermaid_path_clone,
                            &meta_path_clone,
                            &pending_graph_clone,
                        );
                    }
                    _ => {}
                }
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        let mermaid_dir = mermaid_path.parent().unwrap_or(std::path::Path::new("."));
        let meta_dir = meta_path.parent().unwrap_or(std::path::Path::new("."));

        watcher
            .watch(mermaid_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch Mermaid dir: {}", e))?;

        if mermaid_dir != meta_dir {
            watcher
                .watch(meta_dir, RecursiveMode::NonRecursive)
                .map_err(|e| format!("Failed to watch meta dir: {}", e))?;
        }

        Ok(Self {
            _watcher: watcher,
            pending_graph,
        })
    }

    pub fn take_pending(&self) -> Option<PipelineGraph> {
        self.pending_graph.lock().ok()?.take()
    }
}

fn reload_pipeline(
    mermaid_path: &PathBuf,
    meta_path: &PathBuf,
    pending_graph: &Arc<Mutex<Option<PipelineGraph>>>,
) -> Result<(), String> {
    let mermaid_content = std::fs::read_to_string(mermaid_path)
        .map_err(|e| format!("Failed to read {}: {}", mermaid_path.display(), e))?;
    let meta_content = std::fs::read_to_string(meta_path)
        .map_err(|e| format!("Failed to read {}: {}", meta_path.display(), e))?;

    let mermaid_graph = parser::mermaid::parse(&mermaid_content)
        .map_err(|e| format!("Failed to parse Mermaid: {}", e))?;
    let metadata =
        parser::meta::parse(&meta_content).map_err(|e| format!("Failed to parse meta: {}", e))?;

    let graph = parser::build_graph(&mermaid_graph, &metadata);

    if let Ok(mut pending) = pending_graph.lock() {
        *pending = Some(graph);
    }

    Ok(())
}
