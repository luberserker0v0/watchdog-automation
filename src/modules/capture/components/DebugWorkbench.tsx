import type { ComponentType } from "react";
import type { ActionTestContext, PipelineTarget } from "../../../api/pipeline";
import type { CaptureMode } from "../../shell/stores/usePreferencesStore";
import { useDebugStore, type DebugToolId } from "../stores/useDebugStore";
import ActionTesterTab from "../../testing/tabs/ActionTesterTab";
import InspectorTab from "./tabs/InspectorTab";
import RecognitionTab from "../../testing/tabs/RecognitionTab";
import NodeEmulatorTool from "../../testing/emulator/NodeEmulatorTool";

export interface DebugToolContext {
  captureMode: CaptureMode;
  isStreaming: boolean;
  captureBuf: number[] | null;
  hasFrame: boolean;
  currentTarget?: PipelineTarget;
  actionContext?: ActionTestContext;
}

export interface DebugToolDefinition {
  id: DebugToolId;
  label: string;
  component: ComponentType<{ context: DebugToolContext }>;
  requires?: Array<"frame" | "captureTarget" | "actionGeometry">;
}

const CoordinateTool = ({ context }: { context: DebugToolContext }) => <InspectorTab captureMode={context.captureMode} />;
const ActionTool = ({ context }: { context: DebugToolContext }) => <ActionTesterTab context={context.actionContext} />;
const RecognitionTool = ({ context }: { context: DebugToolContext }) => <RecognitionTab isStreaming={context.isStreaming} captureBuf={context.captureBuf} captureMode={context.captureMode} />;
const EmulatorTool = ({ context }: { context: DebugToolContext }) => <NodeEmulatorTool currentTarget={context.currentTarget} />;

export const DEBUG_TOOLS: DebugToolDefinition[] = [
  { id: "inspector", label: "Coordinate", component: CoordinateTool },
  { id: "action", label: "Action", component: ActionTool, requires: ["actionGeometry"] },
  { id: "recognition", label: "Recognition", component: RecognitionTool, requires: ["frame"] },
  { id: "node-emulator", label: "Node Emulator", component: EmulatorTool, requires: ["captureTarget"] },
];

export interface DebugWorkbenchProps {
  context: DebugToolContext;
  captureControl: {
    value: string;
    label?: string;
    disabled?: boolean;
    readOnly?: boolean;
    onChange?: (value: string) => void;
  };
}

function unavailableReason(tool: DebugToolDefinition, context: DebugToolContext): string | null {
  if (tool.requires?.includes("frame") && !context.hasFrame) return "Capture or stream a frame to use this tool.";
  if (tool.requires?.includes("captureTarget") && !context.currentTarget) return "Select a capture target to use this tool.";
  if (tool.requires?.includes("actionGeometry") && !context.actionContext) return "Capture a frame from a selected target to resolve action coordinates.";
  return null;
}

export default function DebugWorkbench({ context, captureControl }: DebugWorkbenchProps) {
  const activeToolId = useDebugStore((state) => state.activeToolId);
  const setActiveToolId = useDebugStore((state) => state.setActiveToolId);
  const activeTool = DEBUG_TOOLS.find((tool) => tool.id === activeToolId) ?? DEBUG_TOOLS[0];
  const ActiveComponent = activeTool.component;
  const unavailable = unavailableReason(activeTool, context);

  return <div className="debug-workbench">
    <div className="debug-workbench-toolbar">
      <select className="debug-tool-select" aria-label="Debug tool" value={activeTool.id} onChange={(event) => setActiveToolId(event.target.value as DebugToolId)}>
        {DEBUG_TOOLS.map((tool) => <option key={tool.id} value={tool.id}>{tool.label}</option>)}
      </select>
      <label className="debug-capture-space"><span>Capture</span>{captureControl.readOnly
        ? <output>{captureControl.label ?? captureControl.value}</output>
        : <select aria-label="Capture area" value={captureControl.value} disabled={captureControl.disabled} onChange={(event) => captureControl.onChange?.(event.target.value)} title={captureControl.disabled ? "Stop streaming before changing capture area" : undefined}><option value="screen">Screen</option><option value="window_frame">Window frame</option><option value="window_client">Window client</option></select>}
      </label>
    </div>
    <div className="debug-tool-layout">
      <nav className="debug-tool-sidebar" aria-label="Debug tools">{DEBUG_TOOLS.map((tool) => <button key={tool.id} className={tool.id === activeTool.id ? "active" : ""} onClick={() => setActiveToolId(tool.id)}>{tool.label}</button>)}</nav>
      <div className="debug-tool-content">{unavailable ? <div className="debug-tool-unavailable">{unavailable}</div> : <ActiveComponent context={context} />}</div>
    </div>
  </div>;
}
