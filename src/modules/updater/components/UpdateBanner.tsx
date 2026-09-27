import { useTranslation } from "react-i18next";
import { useUpdateStore } from "../stores/useUpdateStore";

interface UpdateBannerProps {
  onOpenSettings: () => void;
}

export default function UpdateBanner({ onOpenSettings }: UpdateBannerProps) {
  const { t } = useTranslation();
  const phase = useUpdateStore((state) => state.phase);
  const update = useUpdateStore((state) => state.update);
  if (!update || !["available", "downloading", "installing"].includes(phase)) return null;

  return (
    <div className="update-banner" role="status">
      <div>
        <strong>{t("updater.bannerTitle", { version: update.version })}</strong>
        <span>{t(`updater.phase.${phase}`)}</span>
      </div>
      <button type="button" onClick={onOpenSettings}>{t("updater.viewUpdate")}</button>
    </div>
  );
}
