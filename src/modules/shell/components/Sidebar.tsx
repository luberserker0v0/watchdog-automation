import { useUiStore, type ActiveView } from "../stores/useUiStore";
import { usePreferencesStore } from "../stores/usePreferencesStore";

interface NavItem {
  id: ActiveView;
  icon: string;
  label: string;
}

const NAV_ITEMS: NavItem[] = [
  { id: "pipeline", icon: "🔗", label: "Pipeline" },
  { id: "nodeTest", icon: "🧪", label: "Node-Test" },
  { id: "patrolChat", icon: "💬", label: "Patrol Chat Room" },
];

export default function Sidebar() {
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
          title={sidebarOpen ? "收合" : "展開"}
        >
          <span className="sidebar-toggle-icon">{sidebarOpen ? "◀" : "▶"}</span>
        </button>

        <nav className="sidebar-nav">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              className={`sidebar-nav-item ${activeView === item.id ? "active" : ""}`}
              onClick={() => setActiveView(item.id)}
              title={item.label}
              aria-label={item.label}
            >
              <span className="sidebar-nav-icon">{item.icon}</span>
              {sidebarOpen && <span className="sidebar-nav-label">{item.label}</span>}
            </button>
          ))}
        </nav>
      </div>

      <div className="sidebar-bottom">
        <button
          className={`sidebar-nav-item ${capturePanelVisible ? "active" : ""}`}
          onClick={() => {
            setCapturePanelVisible(!capturePanelVisible);
            if (!capturePanelVisible) setCapturePanelOpen(false);
          }}
          title={capturePanelVisible ? "Hide Capture" : "Show Capture"}
          aria-pressed={capturePanelVisible}
        >
          <span className="sidebar-nav-icon">🖥</span>
          {sidebarOpen && <span className="sidebar-nav-label">Capture</span>}
        </button>
        <button
          className={`sidebar-nav-item ${activeView === "settings" ? "active" : ""}`}
          onClick={() => setActiveView("settings")}
          title="設定"
        >
          <span className="sidebar-nav-icon">⚙</span>
          {sidebarOpen && <span className="sidebar-nav-label">設定</span>}
        </button>
      </div>
    </aside>
  );
}
