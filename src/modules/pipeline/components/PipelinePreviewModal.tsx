import { useEffect, useRef } from "react";
import MermaidGraph from "./MermaidGraph";
import type { PipelineGraph } from "../../../api/pipeline";

interface PipelinePreviewModalProps {
  name: string;
  mermaid: string;
  graph?: PipelineGraph | null;
  onClose: () => void;
}

export default function PipelinePreviewModal({
  name,
  mermaid,
  graph,
  onClose,
}: PipelinePreviewModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  return (
    <div className="pipeline-preview-overlay">
      <div className="pipeline-preview-modal" ref={modalRef}>
        <div className="pipeline-preview-header">
          <span className="pipeline-preview-title">📄 {name}</span>
          <button className="pipeline-preview-close" onClick={onClose}>✕</button>
        </div>
        <div className="pipeline-preview-content">
          <MermaidGraph
            mermaid={mermaid}
            flow={graph}
            currentNodeId={null}
            selectedNodeId={null}
            onNodeClick={() => {}}
          />
        </div>
      </div>
    </div>
  );
}
