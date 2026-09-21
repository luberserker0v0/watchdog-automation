import { useEffect, useRef } from "react";
import type { Node as PipelineNode, StepResult } from "../../../../api/pipeline";

interface NodeDetailModalProps {
  node: PipelineNode;
  hitCount?: number;
  lastStep?: StepResult;
  onClose: () => void;
}

export default function NodeDetailModal({
  node,
  hitCount = 0,
  lastStep,
  onClose,
}: NodeDetailModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (modalRef.current && !modalRef.current.contains(e.target as globalThis.Node)) {
        onClose();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  const maxHit = node.max_hit;
  const maxHitDefined = maxHit !== undefined && maxHit < 2147483647;

  return (
    <div className="pipeline-preview-overlay">
      <div className="node-detail-modal" ref={modalRef}>
        <div className="pipeline-preview-header">
          <span className="pipeline-preview-title">
            {node.label || node.id}
          </span>
          <button className="pipeline-preview-close" onClick={onClose}>✕</button>
        </div>
        {node.desc && (
          <div className="node-detail-desc">{node.desc}</div>
        )}
        <div className="node-detail-content">
          <section className="node-detail-section">
            <h4>Recognition</h4>
            <div className="node-detail-grid">
              <span className="node-detail-label">Type</span>
              <span className="node-detail-value">{node.recognition.type}</span>
              {"expected" in node.recognition && (
                <>
                  <span className="node-detail-label">Expected</span>
                  <span className="node-detail-value node-detail-mono">
                    {String((node.recognition as Record<string, unknown>).expected)}
                  </span>
                </>
              )}
              {"threshold" in node.recognition && (
                <>
                  <span className="node-detail-label">Threshold</span>
                  <span className="node-detail-value">
                    {String((node.recognition as Record<string, unknown>).threshold)}
                  </span>
                </>
              )}
            </div>
          </section>

          {node.actions && node.actions.length > 0 && (
            <section className="node-detail-section">
              <h4>Actions ({node.actions.length})</h4>
              <div className="node-detail-actions">
                {node.actions.map((action, i) => (
                  <span key={i} className="node-detail-action-badge">{action.type}</span>
                ))}
              </div>
            </section>
          )}

          <section className="node-detail-section">
            <h4>Timing</h4>
            <div className="node-detail-grid">
              <span className="node-detail-label">Timeout</span>
              <span className="node-detail-value">{node.timeout ?? 20000}ms</span>
              <span className="node-detail-label">Rate limit</span>
              <span className="node-detail-value">{node.rate_limit ?? 1000}ms</span>
              <span className="node-detail-label">Pre-delay</span>
              <span className="node-detail-value">{node.pre_delay ?? 200}ms</span>
              <span className="node-detail-label">Post-delay</span>
              <span className="node-detail-value">{node.post_delay ?? 200}ms</span>
            </div>
          </section>

          <section className="node-detail-section">
            <h4>Control</h4>
            <div className="node-detail-grid">
              <span className="node-detail-label">Enabled</span>
              <span className="node-detail-value">{node.enabled !== false ? "Yes" : "No"}</span>
              <span className="node-detail-label">Inverse</span>
              <span className="node-detail-value">{node.inverse ? "Yes" : "No"}</span>
              <span className="node-detail-label">Repeat</span>
              <span className="node-detail-value">{node.repeat ?? 1}</span>
              {maxHitDefined && (
                <>
                  <span className="node-detail-label">Max hit</span>
                  <span className="node-detail-value">{maxHit}</span>
                </>
              )}
            </div>
          </section>

          <section className="node-detail-section">
            <h4>Runtime</h4>
            <div className="node-detail-grid">
              <span className="node-detail-label">Hit count</span>
              <span className="node-detail-value">
                {maxHitDefined ? `${hitCount} / ${maxHit}` : hitCount}
              </span>
              {lastStep && (
                <>
                  <span className="node-detail-label">Last match</span>
                  <span className="node-detail-value">
                    {lastStep.matched ? "Yes" : "No"}
                    {lastStep.confidence > 0 ? ` (${(lastStep.confidence * 100).toFixed(1)}%)` : ""}
                  </span>
                  <span className="node-detail-label">Last duration</span>
                  <span className="node-detail-value">{lastStep.duration_ms}ms</span>
                  {lastStep.error && (
                    <>
                      <span className="node-detail-label">Error</span>
                      <span className="node-detail-value node-detail-error">{lastStep.error}</span>
                    </>
                  )}
                </>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
