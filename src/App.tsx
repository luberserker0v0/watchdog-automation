import { useEffect } from "react";
import { setRecognitionDevice } from "./api/pipeline";
import { useUiStore } from "./modules/core/stores/useUiStore";
import { usePreferencesStore } from "./modules/core/stores/usePreferencesStore";
import Sidebar from "./modules/core/components/Sidebar";
import ResizableSplit from "./modules/core/components/ResizableSplit";
import CaptureView from "./modules/capture/views/CaptureView";
import PipelineView from "./modules/pipeline/views/PipelineView";
import NodeTestView from "./modules/pipeline/views/NodeTestView";
import ExecutionLog from "./modules/pipeline/components/ExecutionLog";
import PreviewPage from "./modules/capture/views/PreviewPage";
import SettingsView from "./modules/settings/views/SettingsView";
import { useCaptureController } from "./modules/capture/hooks/useCaptureController";
import "./modules/core/styles/sidebar.css";
import "./App.css";

function SplitLayout() {
  const capturePanelOpen = useUiStore((state) => state.capturePanelOpen);
  const capturePanelVisible = usePreferencesStore((state) => state.capturePanelVisible);
  return (
    <ResizableSplit
      left={<>
        <Workspace />
        <ExecutionLog />
      </>}
      right={<CaptureView />}
      rightCollapsed={!capturePanelOpen}
      rightHidden={!capturePanelVisible}
    />
  );
}

function Workspace() {
  const activeView = useUiStore((s) => s.activeView);

  return (
    <div className="workspace">
      {activeView === "pipeline" && <PipelineView />}
      {activeView === "nodeTest" && <NodeTestView />}
    </div>
  );
}

function MainContent() {
  const activeView = useUiStore((s) => s.activeView);

  return (
    <div className="app-content">
      {activeView === "pipeline" && <SplitLayout />}
      {activeView === "nodeTest" && <SplitLayout />}
      {activeView === "settings" && <SettingsView />}
    </div>
  );
}

function App() {
  const isPreview = window.location.hash === "#preview";
  const recognitionDevice = usePreferencesStore((state) => state.recognitionDevice);
  useCaptureController(!isPreview);

  useEffect(() => {
    if (isPreview) return;
    void setRecognitionDevice(recognitionDevice)
      .then((activeDevice) => {
        console.info(`[ocr] active recognition device: ${activeDevice}`);
      })
      .catch((error) => {
        console.error(`[ocr] failed to configure recognition device: ${error}`);
      });
  }, [isPreview, recognitionDevice]);

  if (isPreview) {
    return <PreviewPage />;
  }

  return (
    <div className="app-layout">
      <Sidebar />
      <MainContent />
    </div>
  );
}

export default App;
