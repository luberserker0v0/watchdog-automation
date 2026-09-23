import { invoke } from "@tauri-apps/api/core";
import type { LoadResult, PipelineGraph, PipelineInfo, PipelineNodePosition, ValidationError } from "./models";

export async function loadPipeline(path: string): Promise<LoadResult> { return invoke("load_pipeline", { path }); }
export async function savePipeline(path: string, mermaid: string, meta: string): Promise<boolean> { return invoke("save_pipeline", { path, mermaid, meta }); }
export async function savePipelineLayout(path: string, positions: Record<string, PipelineNodePosition>): Promise<boolean> { return invoke("save_pipeline_layout", { path, positions }); }
export async function validatePipeline(graph: PipelineGraph): Promise<ValidationError[]> { return invoke("validate_pipeline", { graph }); }
export async function listPipelines(): Promise<PipelineInfo[]> { return invoke("list_pipelines"); }
