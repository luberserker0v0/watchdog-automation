import { useEffect, useState } from "react";
import { listRelatedFolders, openRelatedFolder, type RelatedFolder } from "../../../api/settings";
import { useTranslation } from "react-i18next";

export default function RelatedDetailsSection() {
  const { t } = useTranslation();
  const [folders, setFolders] = useState<RelatedFolder[]>([]);
  const [error, setError] = useState("");
  const [opening, setOpening] = useState<string | null>(null);

  useEffect(() => {
    void listRelatedFolders()
      .then(setFolders)
      .catch((reason) => setError(t("folders.loadError", { error: String(reason) })));
  }, [t]);

  const open = async (folder: RelatedFolder) => {
    setOpening(folder.key);
    setError("");
    try {
      await openRelatedFolder(folder.key);
    } catch (reason) {
      setError(t("folders.openError", { name: folder.name, error: String(reason) }));
    } finally {
      setOpening(null);
    }
  };

  return (
    <div className="settings-section">
      <h3 className="settings-section-title">{t("settings.related")}</h3>
      <div className="settings-folder-list">
        {folders.map((folder) => (
          <div className="settings-folder" key={folder.key}>
            <div className="settings-folder-detail">
              <div><strong>{folder.name}</strong><span aria-hidden="true">:</span> {t(`folderPurposes.${folder.key}`, { defaultValue: folder.purpose })}</div>
              <code title={folder.path}>{folder.path}</code>
            </div>
            <button className="settings-action" type="button" disabled={opening !== null}
              onClick={() => void open(folder)} aria-label={t("folders.openLabel", { name: folder.name })}>
              {opening === folder.key ? t("common.opening") : t("common.openFolder")}
            </button>
          </div>
        ))}
        {!folders.length && !error && <span className="settings-item-hint">{t("folders.loading")}</span>}
        {error && <span className="settings-item-error" role="alert">{error}</span>}
      </div>
    </div>
  );
}
