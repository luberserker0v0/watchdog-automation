import { DashboardContent } from "./CombinedPage";

export default function DashboardPage() {
  return (
    <div className="target-page target-page-scroll">
      <div className="target-app">
        <div className="target-app-header">
          <div className="target-app-logo">
            <div className="target-app-logo-icon">
              <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
            </div>
            <span className="target-app-logo-text">WatchDog</span>
          </div>
          <div className="target-app-user">
            <div className="target-app-avatar">A</div>
          </div>
        </div>
        <div className="target-app-body">
          <DashboardContent />
        </div>
      </div>
    </div>
  );
}
