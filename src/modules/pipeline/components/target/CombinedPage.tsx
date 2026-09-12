import { useState, useRef, useEffect } from "react";
import CombinedLogin from "./CombinedLogin";

// ─── SVG Icons ───
function ShieldIcon() {
  return (
    <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  );
}

function BellIcon() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

function RefreshIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="23 4 23 10 17 10" />
      <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
    </svg>
  );
}

function ShareIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" />
      <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" /><line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
    </svg>
  );
}

function BookmarkIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
    </svg>
  );
}

// ─── Data ───
const CARDS = [
  { id: 1, title: "Card 1", color: "#6366f1", desc: "Swipe left to see more" },
  { id: 2, title: "Card 2", color: "#22c55e", desc: "Keep swiping..." },
  { id: 3, title: "Card 3", color: "#f59e0b", desc: "Almost there" },
  { id: 4, title: "Card 4", color: "#ef4444", desc: "Last card!" },
];

const AVATAR_COLORS = ["#6366f1", "#ec4899", "#14b8a6", "#f59e0b", "#8b5cf6", "#06b6d4", "#ef4444", "#22c55e"];

const ITEMS = Array.from({ length: 25 }, (_, i) => ({
  id: i + 1,
  title: `Notification ${i + 1}`,
  desc: [
    "New message from Alice",
    "System update available",
    "Server maintenance at 3 AM",
    "New comment on your post",
    "Payment received",
    "Friend request from Bob",
    "Password changed successfully",
    "New login from Chrome/Windows",
    "Weekly report ready",
    "Meeting in 15 minutes",
  ][i % 10],
  time: `${String(i + 1).padStart(2, "0")}:${String((i * 7) % 60).padStart(2, "0")}`,
  color: AVATAR_COLORS[i % AVATAR_COLORS.length],
}));

// ─── Dashboard Content ───
export function DashboardContent() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const carouselRef = useRef<HTMLDivElement>(null);
  const carouselCounterRef = useRef<HTMLSpanElement>(null);
  const carouselDrag = useRef({ pointerId: -1, startX: 0, scrollLeft: 0 });
  const carouselMouseCleanup = useRef<(() => void) | null>(null);
  const carouselPointerCleanup = useRef<(() => void) | null>(null);
  const carouselFrame = useRef<number | null>(null);
  const [longPressing, setLongPressing] = useState(false);
  const [longPressDone, setLongPressDone] = useState(false);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [lastKey, setLastKey] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      if (longPressTimer.current) clearTimeout(longPressTimer.current);
      if (carouselFrame.current !== null) cancelAnimationFrame(carouselFrame.current);
      carouselMouseCleanup.current?.();
      carouselPointerCleanup.current?.();
    };
  }, []);

  useEffect(() => {
    if (!lastKey) return;
    const t = setTimeout(() => setLastKey(null), 1200);
    return () => clearTimeout(t);
  }, [lastKey]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => setLastKey(event.key);
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleLongPressStart = () => {
    setLongPressing(true);
    setLongPressDone(false);
    longPressTimer.current = setTimeout(() => {
      setLongPressDone(true);
      setLongPressing(false);
    }, 1500);
  };

  const handleLongPressEnd = () => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
    if (!longPressDone) {
      setLongPressing(false);
    }
  };

  const updateCarouselCounter = () => {
    if (carouselFrame.current !== null) cancelAnimationFrame(carouselFrame.current);
    carouselFrame.current = requestAnimationFrame(() => {
      carouselFrame.current = null;
      const carousel = carouselRef.current;
      const counter = carouselCounterRef.current;
      if (!carousel || !counter) return;
      const cards = Array.from(carousel.children) as HTMLElement[];
      const activeIndex = cards.reduce((best, card, index) => (
        Math.abs(card.offsetLeft - carousel.scrollLeft)
          < Math.abs(cards[best].offsetLeft - carousel.scrollLeft) ? index : best
      ), 0);
      counter.textContent = `${activeIndex + 1} / ${cards.length}`;
    });
  };

  const beginCarouselDrag = (carousel: HTMLDivElement, pointerId: number, clientX: number) => {
    carouselDrag.current = {
      pointerId,
      startX: clientX,
      scrollLeft: carousel.scrollLeft,
    };
    carousel.classList.add("dragging");
    carousel.style.scrollSnapType = "none";
  };

  const moveCarouselDrag = (carousel: HTMLDivElement, pointerId: number, clientX: number) => {
    if (carouselDrag.current.pointerId !== pointerId) return;
    carousel.scrollLeft = carouselDrag.current.scrollLeft
      - (clientX - carouselDrag.current.startX);
  };

  const finishCarouselDrag = (carousel: HTMLDivElement, pointerId: number) => {
    if (carouselDrag.current.pointerId !== pointerId) return;
    carouselDrag.current.pointerId = -1;
    carousel.classList.remove("dragging");
    carousel.style.scrollSnapType = "";
    const cards = Array.from(carousel.children) as HTMLElement[];
    const nearest = cards.reduce((best, card, index) => (
      Math.abs(card.offsetLeft - carousel.scrollLeft)
        < Math.abs(cards[best].offsetLeft - carousel.scrollLeft) ? index : best
    ), 0);
    carousel.scrollTo({ left: cards[nearest]?.offsetLeft ?? 0, behavior: "smooth" });
    updateCarouselCounter();
  };

  const handleCarouselPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    const carousel = event.currentTarget;
    carouselPointerCleanup.current?.();
    beginCarouselDrag(carousel, event.pointerId, event.clientX);
    if (typeof carousel.setPointerCapture === "function") {
      try {
        carousel.setPointerCapture(event.pointerId);
      } catch {
        // Movement still works while the pointer remains over the carousel.
      }
    }

    const pointerId = event.pointerId;
    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (moveEvent.pointerId !== pointerId) return;
      moveEvent.preventDefault();
      moveCarouselDrag(carousel, pointerId, moveEvent.clientX);
    };
    const cleanup = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerEnd);
      window.removeEventListener("pointercancel", handlePointerEnd);
      carouselPointerCleanup.current = null;
    };
    const handlePointerEnd = (endEvent: PointerEvent) => {
      if (endEvent.pointerId !== pointerId) return;
      finishCarouselDrag(carousel, pointerId);
      cleanup();
    };
    window.addEventListener("pointermove", handlePointerMove, { passive: false });
    window.addEventListener("pointerup", handlePointerEnd);
    window.addEventListener("pointercancel", handlePointerEnd);
    carouselPointerCleanup.current = cleanup;
  };

  const handleCarouselPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (carouselDrag.current.pointerId !== event.pointerId) return;
    event.preventDefault();
    moveCarouselDrag(event.currentTarget, event.pointerId, event.clientX);
  };

  const handleCarouselPointerEnd = (event: React.PointerEvent<HTMLDivElement>) => {
    const carousel = event.currentTarget;
    if (typeof carousel.hasPointerCapture === "function"
      && carousel.hasPointerCapture(event.pointerId)) {
      carousel.releasePointerCapture(event.pointerId);
    }
    finishCarouselDrag(carousel, event.pointerId);
    carouselPointerCleanup.current?.();
  };

  const handleCarouselMouseDown = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    // Pointer-enabled browsers already installed the stronger window-level
    // pointer gesture. This is only a fallback for older WebViews.
    if (carouselDrag.current.pointerId !== -1) return;
    event.preventDefault();
    const carousel = event.currentTarget;
    const mouseDragId = -2;
    carouselMouseCleanup.current?.();
    beginCarouselDrag(carousel, mouseDragId, event.clientX);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      moveEvent.preventDefault();
      moveCarouselDrag(carousel, mouseDragId, moveEvent.clientX);
    };
    const cleanup = () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      carouselMouseCleanup.current = null;
    };
    const handleMouseUp = () => {
      finishCarouselDrag(carousel, mouseDragId);
      cleanup();
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    carouselMouseCleanup.current = cleanup;
  };

  const initials = (name: string) => name.split(" ").map(w => w[0]).join("").substring(0, 2).toUpperCase();

  return (
    <div className="target-dashboard">
      <div className="target-title" data-target="text:Dashboard">Dashboard</div>

      <div>
        <div className="target-section-header">
          <span className="target-settings-label" data-target="text:Featured">Featured</span>
          <span ref={carouselCounterRef} className="target-carousel-counter">1 / {CARDS.length}</span>
        </div>
        <div
          ref={carouselRef}
          className="target-carousel"
          data-target="swipe:carousel"
          onMouseDown={handleCarouselMouseDown}
          onPointerDown={handleCarouselPointerDown}
          onPointerMove={handleCarouselPointerMove}
          onPointerUp={handleCarouselPointerEnd}
          onPointerCancel={handleCarouselPointerEnd}
          onDragStart={(event) => event.preventDefault()}
          onSelect={(event) => event.preventDefault()}
          onScroll={updateCarouselCounter}
        >
          {CARDS.map((card) => (
            <div
              key={card.id}
              className="target-carousel-card"
              data-target={`card:${card.id}`}
              style={{ backgroundColor: card.color }}
            >
              <div className="target-carousel-title">{card.title}</div>
              <div className="target-carousel-desc">{card.desc}</div>
            </div>
          ))}
        </div>
      </div>

      <div>
        <div className="target-section-header">
          <span className="target-settings-label" data-target="text:Notifications">Notifications</span>
        </div>
        <div className="target-scroll-list" data-target="scroll:list" ref={scrollRef}>
          {ITEMS.map((item) => (
            <div key={item.id} className="target-scroll-item" data-target={`item:${item.id}`}>
              <div className="target-scroll-item-avatar" style={{ background: item.color }}>
                {initials(item.title)}
              </div>
              <div className="target-scroll-item-content">
                <div className="target-scroll-item-title">{item.title}</div>
                <div className="target-scroll-item-desc">{item.desc}</div>
              </div>
              <div className="target-scroll-item-time">{item.time}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="target-longpress-section">
        <button
          className={`target-longpress-btn ${longPressing ? "pressing" : ""} ${longPressDone ? "done" : ""}`}
          data-target="button:delete"
          onMouseDown={handleLongPressStart}
          onMouseUp={handleLongPressEnd}
          onMouseLeave={handleLongPressEnd}
          onTouchStart={handleLongPressStart}
          onTouchEnd={handleLongPressEnd}
        >
          <svg className="target-longpress-ring" viewBox="0 0 72 72">
            <circle cx="36" cy="36" r="34" />
          </svg>
          {longPressDone ? "\u2713" : <TrashIcon />}
        </button>
        <span className="target-longpress-label">
          {longPressDone ? "Deleted" : longPressing ? "Hold..." : "Hold to Delete"}
        </span>
      </div>

      <div className="target-actions-row">
        <button className="target-action-btn" data-target="button:refresh" onClick={() => setLongPressDone(false)}>
          <RefreshIcon /> Refresh
        </button>
        <button className="target-action-btn" data-target="button:share">
          <ShareIcon /> Share
        </button>
        <button className="target-action-btn" data-target="button:bookmark">
          <BookmarkIcon /> Save
        </button>
      </div>

      <div>
        <div className="target-section-header">
          <span className="target-settings-label" data-target="text:Key Shortcuts">Key Shortcuts</span>
          {lastKey && <span className="target-key-flash">{lastKey}</span>}
        </div>
        <div className="target-key-row">
          {(["Enter", "Space", "Tab", "Escape", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"] as const).map((k) => (
            <button
              key={k}
              className={`target-key-btn ${lastKey === k ? "active" : ""}`}
              data-target={`key:${k}`}
              onClick={() => setLastKey(k)}
            >
              {k}
            </button>
          ))}
        </div>
      </div>

      <div>
        <div className="target-section-header">
          <span className="target-settings-label" data-target="text:Vertical Swipe">Vertical Swipe</span>
        </div>
        <div className="target-swipe-vertical" data-target="swipe:vertical">
          {["Drag Up", "Slide Me", "Swipe Down", "Keep Going", "Almost Done"].map((label, i) => (
            <div key={label} className="target-swipe-v-card" data-target={`vcard:${i}`} style={{ background: AVATAR_COLORS[i] }}>
              {label}
            </div>
          ))}
        </div>
      </div>

      <div>
        <div className="target-section-header">
          <span className="target-settings-label" data-target="text:Horizontal Scroll">Horizontal Scroll</span>
        </div>
        <div className="target-scroll-horizontal" data-target="scroll:horizontal">
          {Array.from({ length: 8 }, (_, i) => (
            <div key={`horizontal-item-${i + 1}`} className="target-scroll-h-item" data-target={`hitem:${i}`}>
              <div className="target-scroll-h-icon" style={{ background: AVATAR_COLORS[i] }}>
                {String(i + 1).padStart(2, "0")}
              </div>
              <span className="target-scroll-h-label">Item {i + 1}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ─── Settings Content ───
export function SettingsContent() {
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [notifications, setNotifications] = useState(true);
  const [darkMode, setDarkMode] = useState(false);
  const [status, setStatus] = useState("online");
  const [saved, setSaved] = useState(false);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => {
    if (savedTimer.current) clearTimeout(savedTimer.current);
  }, []);

  const handleSave = () => {
    setSaved(true);
    if (savedTimer.current) clearTimeout(savedTimer.current);
    savedTimer.current = setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="target-settings">
      <div className="target-settings-header" data-target="icon:gear">
        <svg className="target-icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
        <span className="target-title" data-target="text:Settings">Settings</span>
      </div>

      <div className="target-settings-section">
        <div className="target-settings-section-title" data-target="text:Profile">Profile</div>
        <div className="target-profile-row">
          <div className="target-profile-avatar">A</div>
          <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
            <div className="target-field">
              <label className="target-label" data-target="text:Display Name">Display Name</label>
              <div className="target-input-wrap">
                <input
                  className="target-input"
                  data-target="input:display_name"
                  type="text"
                  placeholder="Enter display name..."
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                />
              </div>
            </div>
            <div className="target-field">
              <label className="target-label" data-target="text:Email">Email</label>
              <div className="target-input-wrap">
                <input
                  className="target-input"
                  data-target="input:email"
                  type="email"
                  placeholder="Enter email..."
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="target-settings-section">
        <div className="target-settings-section-title" data-target="text:Status">Status</div>
        <div className="target-status-options">
          {(["online", "away", "offline", "busy"] as const).map((s) => (
            <label key={s} className="target-status-option">
              <span className={`target-status-dot target-status-${s}`} data-target={`color:${s}`} />
              <input type="radio" name="status" value={s} checked={status === s} onChange={() => setStatus(s)} />
              <span className="target-status-text">{s.charAt(0).toUpperCase() + s.slice(1)}</span>
            </label>
          ))}
        </div>
      </div>

      <div className="target-settings-section">
        <div className="target-settings-section-title" data-target="text:Color Palette">Color Palette</div>
        <div className="target-color-palette">
          {[
            { name: "red", hex: "#ef4444" },
            { name: "orange", hex: "#f97316" },
            { name: "amber", hex: "#f59e0b" },
            { name: "emerald", hex: "#10b981" },
            { name: "cyan", hex: "#06b6d4" },
            { name: "indigo", hex: "#6366f1" },
            { name: "violet", hex: "#8b5cf6" },
            { name: "pink", hex: "#ec4899" },
          ].map((c) => (
            <div key={c.name} className="target-color-swatch" data-target={`palette:${c.name}`}>
              <div className="target-color-block" style={{ background: c.hex }} />
              <span className="target-color-name">{c.name}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="target-settings-section">
        <div className="target-settings-section-title" data-target="text:Preferences">Preferences</div>
        <div className="target-toggle-row">
          <div className="target-toggle-left">
            <BellIcon />
            <span data-target="text:Notifications">Notifications</span>
          </div>
          <button
            className={`target-toggle ${notifications ? "on" : "off"}`}
            data-target="toggle:notifications"
            onClick={() => setNotifications(!notifications)}
          >
            <span className="target-toggle-knob" />
          </button>
        </div>
        <div className="target-toggle-row">
          <div className="target-toggle-left">
            <MoonIcon />
            <span data-target="text:Dark Mode">Dark Mode</span>
          </div>
          <button
            className={`target-toggle ${darkMode ? "on" : "off"}`}
            data-target="toggle:dark_mode"
            onClick={() => setDarkMode(!darkMode)}
          >
            <span className="target-toggle-knob" />
          </button>
        </div>
      </div>

      <div className="target-settings-section">
        <div className="target-settings-section-title">Shortcuts</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <span className="target-shortcut-hint" data-target="text:Ctrl+S">Ctrl+S to Save</span>
          <span className="target-shortcut-hint" data-target="text:Ctrl+R">Ctrl+R to Reset</span>
        </div>
      </div>

      <div className="target-settings-actions">
        <button className="target-button" data-target="button:save" onClick={handleSave}>
          {saved ? "Saved!" : "Save"}
        </button>
        <button
          className="target-button-secondary"
          data-target="button:reset"
          onClick={() => {
            setDisplayName("");
            setEmail("");
            setNotifications(true);
            setDarkMode(false);
            setStatus("online");
          }}
        >
          Reset
        </button>
      </div>

      <div className="target-settings-footer">
        <span data-target="text:App Version">App Version: 1.0.0</span>
      </div>
    </div>
  );
}

// ─── Combined Page ───
export default function CombinedPage() {
  const [view, setView] = useState<"login" | "dashboard">("login");
  const [dashboardTab, setDashboardTab] = useState<"dashboard" | "settings">("dashboard");

  const handleLogout = () => {
    setView("login");
  };

  if (view === "login") {
    return <CombinedLogin onSuccess={() => { setView("dashboard"); setDashboardTab("dashboard"); }} />;
  }

  return (
    <div className="target-page target-page-scroll">
      <div className="target-app">
        <div className="target-app-header">
          <div className="target-app-logo">
            <div className="target-app-logo-icon">
              <ShieldIcon />
            </div>
            <span className="target-app-logo-text">WatchDog</span>
          </div>
          <div className="target-app-user">
            <div className="target-app-avatar">A</div>
            <button className="target-logout-btn-sm" data-target="button:logout" onClick={handleLogout}>
              Logout
            </button>
          </div>
        </div>

        <div className="target-app-tabs">
          <button
            className={`target-app-tab ${dashboardTab === "dashboard" ? "active" : ""}`}
            data-target="tab:dashboard"
            onClick={() => setDashboardTab("dashboard")}
          >
            Dashboard
          </button>
          <button
            className={`target-app-tab ${dashboardTab === "settings" ? "active" : ""}`}
            data-target="tab:settings"
            onClick={() => setDashboardTab("settings")}
          >
            Settings
          </button>
        </div>

        <div className="target-app-body">
          {dashboardTab === "dashboard" && <DashboardContent />}
          {dashboardTab === "settings" && <SettingsContent />}
        </div>
      </div>
    </div>
  );
}
