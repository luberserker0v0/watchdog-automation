import { useEffect } from "react";
import { usePipelineStore } from "../../pipeline/runtime/stores/usePipelineStore";
import { useRuntimeIntegrationStore } from "../stores/useRuntimeIntegrationStore";

function publishPipelineCaptureContext(): void {
  const state = usePipelineStore.getState();
  const activeEntry = state.activeId ? state.pipelinesById[state.activeId] : undefined;
  const target = activeEntry?.graph?.target;
  useRuntimeIntegrationStore.getState().publishCaptureContext({
    pipelineExecuting: state.isExecuting,
    expectedWindowSize: target?.type === "window" ? target.window_size ?? null : null,
    resizeBeforeSize: state.resizeIssue?.before ?? null,
  });
}

export default function RuntimeIntegrationCoordinator() {
  useEffect(() => {
    publishPipelineCaptureContext();
    return usePipelineStore.subscribe(publishPipelineCaptureContext);
  }, []);
  return null;
}
