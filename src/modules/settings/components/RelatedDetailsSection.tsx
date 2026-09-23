import { useEffect, useState } from "react";
import { listRelatedFolders, openRelatedFolder, type RelatedFolder } from "../../../api/settings";

export default function RelatedDetailsSection() {
  const [folders, setFolders] = useState<RelatedFolder[]>([]);
  const [error, setError] = useState("");
  const [opening, setOpening] = useState<string | null>(null);

  useEffect(() => {
    void listRelatedFolders()
      .then(setFolders)
      .catch((reason) => setError(`Unable to load application folders: ${String(reason)}`));
  }, []);

  const open = async (folder: RelatedFolder) => {
    setOpening(folder.key);
    setError("");
    try {
      await openRelatedFolder(folder.key);
    } catch (reason) {
      setError(`Unable to open ${folder.name}: ${String(reason)}`);
    } finally {
      setOpening(null);
    }
  };

  return (
    <div className="settings-section">
      <h3 className="settings-section-title">Related details</h3>
      <div className="settings-folder-list">
        {folders.map((folder) => (
          <div className="settings-folder" key={folder.key}>
            <div className="settings-folder-detail">
              <div><strong>{folder.name}</strong><span aria-hidden="true">:</span> {folder.purpose}</div>
              <code title={folder.path}>{folder.path}</code>
            </div>
            <button className="settings-action" type="button" disabled={opening !== null}
              onClick={() => void open(folder)} aria-label={`Open ${folder.name} folder`}>
              {opening === folder.key ? "Opening…" : "Open folder"}
            </button>
          </div>
        ))}
        {!folders.length && !error && <span className="settings-item-hint">Loading application folders…</span>}
        {error && <span className="settings-item-error" role="alert">{error}</span>}
      </div>
    </div>
  );
}
