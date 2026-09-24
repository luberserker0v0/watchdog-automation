import { useUiStore, type ActiveView } from "../stores/useUiStore";
import { usePreferencesStore } from "../stores/usePreferencesStore";
import { useTranslation } from "react-i18next";

interface NavItem {
  id: ActiveView;
  icon: string;
  labelKey: string;
}

const NAV_ITEMS: NavItem[] = [
  { id: "pipeline", icon: "🔗", labelKey: "nav.pipeline" },
  { id: "nodeTest", icon: "🧪", labelKey: "nav.nodeTest" },
  { id: "patrolChat", icon: "💬", labelKey: "nav.patrol" },
  { id: "actionRecorder", icon: "⏺", labelKey: "nav.recorder" },
];

interface SidebarProps {
  showNodeTest?: boolean;
}

export default function Sidebar({ showNodeTest = false }: SidebarProps) {
  const { t } = useTranslation();
  const activeView = useUiStore((s) => s.activeView);
  const setActiveView = useUiStore((s) => s.setActiveView);
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const capturePanelVisible = usePreferencesStore((state) => state.capturePanelVisible);
  const setCapturePanelVisible = usePreferencesStore((state) => state.setCapturePanelVisible);
  const setCapturePanelOpen = useUiStore((state) => state.setCapturePanelOpen);

  return (
    <aside className={`sidebar ${sidebarOpen ? "expanded" : ""}`}>
      <div className="sidebar-top">
        <button
          className="sidebar-toggle"
          onClick={toggleSidebar}
          title={sidebarOpen ? t("nav.collapse") : t("nav.expand")}
        >
          <span className="sidebar-toggle-icon">{sidebarOpen ? "◀" : "▶"}</span>
        </button>

        <nav className="sidebar-nav">
          {NAV_ITEMS.filter((item) => showNodeTest || item.id !== "nodeTest").map((item) => {
            const label = t(item.labelKey);
            return (
            <button
              key={item.id}
              className={`sidebar-nav-item ${activeView === item.id ? "active" : ""}`}
              onClick={() => setActiveView(item.id)}
              title={label}
              aria-label={label}
            >
              <span className="sidebar-nav-icon">{item.icon}</span>
              {sidebarOpen && <span className="sidebar-nav-label">{label}</span>}
            </button>
          );})}
        </nav>
      </div>

      <div className="sidebar-bottom">
        <button
          className={`sidebar-nav-item ${capturePanelVisible ? "active" : ""}`}
          onClick={() => {
            setCapturePanelVisible(!capturePanelVisible);
            if (!capturePanelVisible) setCapturePanelOpen(false);
          }}
          title={capturePanelVisible ? t("nav.hideCapture") : t("nav.showCapture")}
          aria-pressed={capturePanelVisible}
        >
          <span className="sidebar-nav-icon">🖥</span>
          {sidebarOpen && <span className="sidebar-nav-label">{t("nav.capture")}</span>}
        </button>
        <button
          className={`sidebar-nav-item ${activeView === "settings" ? "active" : ""}`}
          onClick={() => setActiveView("settings")}
          title={t("nav.settings")}
        >
          <span className="sidebar-nav-icon">⚙</span>
          {sidebarOpen && <span className="sidebar-nav-label">{t("nav.settings")}</span>}
        </button>
      </div>
    </aside>
  );
}
