import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getStartupProgress, initializeApp, type StartupProgress } from "../../../api/startup";
import "../styles/splash.css";

const INITIAL_PROGRESS: StartupProgress = {
  percent: 4,
  stage: "preparing",
  complete: false,
};

const COPY = {
  en: {
    subtitle: "Visual automation, with guarded recovery",
    preparing: "Preparing WatchDog",
    storage: "Preparing application storage",
    migration: "Migrating existing WatchDog data",
    maintenance: "Cleaning generated data",
    desktop: "Loading desktop services",
    schemas: "Compiling pipeline schemas",
    pipelines: "Scanning pipeline resources",
    ocr: "Loading OCR engine",
    ocr_ready: "OCR engine ready",
    interface: "Preparing the interface",
    ready: "Ready",
    failed: "Startup completed with a warning",
    pipelineCount: (count: string) => `${count} pipelines found`,
  },
  "zh-TW": {
    subtitle: "具備受保護復原機制的視覺自動化",
    preparing: "正在準備 WatchDog",
    storage: "正在準備應用程式儲存空間",
    migration: "正在移轉既有 WatchDog 資料",
    maintenance: "正在清理產生的資料",
    desktop: "正在載入桌面服務",
    schemas: "正在編譯流程結構描述",
    pipelines: "正在掃描流程資源",
    ocr: "正在載入 OCR 引擎",
    ocr_ready: "OCR 引擎已就緒",
    interface: "正在準備介面",
    ready: "準備完成",
    failed: "啟動完成，但有一項警告",
    pipelineCount: (count: string) => `找到 ${count} 個流程`,
  },
} as const;

export function readRecognitionDevice(storage: Pick<Storage, "getItem"> = localStorage): "cpu" | "directml" {
  try {
    const raw = storage.getItem("watchdog:settings");
    const device = raw ? JSON.parse(raw)?.state?.recognitionDevice : null;
    return device === "directml" ? "directml" : "cpu";
  } catch {
    return "cpu";
  }
}

function displayDetail(progress: StartupProgress, language: keyof typeof COPY): string | null {
  if (!progress.detail) return null;
  if (progress.stage === "pipelines" && /^\d+$/.test(progress.detail)) {
    return COPY[language].pipelineCount(progress.detail);
  }
  if (progress.stage === "ocr_ready" && /^(cpu|directml)$/i.test(progress.detail)) {
    return progress.detail.toUpperCase();
  }
  return COPY[language].failed;
}

interface SplashScreenProps {
  language?: keyof typeof COPY;
  recognitionDevice?: "cpu" | "directml";
}

export default function SplashScreen({
  language = localStorage.getItem("watchdog:language") === "zh-TW" ? "zh-TW" : "en",
  recognitionDevice = readRecognitionDevice(),
}: SplashScreenProps) {
  const copy = COPY[language];
  const [progress, setProgress] = useState(INITIAL_PROGRESS);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void listen<StartupProgress>("startup-progress", ({ payload }) => {
      if (!disposed) setProgress(payload);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
      return getStartupProgress();
    }).then((snapshot) => {
      if (!disposed) setProgress(snapshot);
      return initializeApp(recognitionDevice);
    }).then((finalProgress) => {
      if (!disposed) setProgress(finalProgress);
    }).catch((error) => {
      if (!disposed) {
        setProgress((current) => ({
          ...current,
          detail: error instanceof Error ? error.message : String(error),
        }));
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [recognitionDevice]);

  const stageLabel = copy[progress.stage as keyof typeof copy];
  const label = typeof stageLabel === "string" ? stageLabel : copy.preparing;
  const detail = displayDetail(progress, language);

  return (
    <main className="splash-shell">
      <div className="splash-glow splash-glow-one" />
      <div className="splash-glow splash-glow-two" />
      <section className="splash-card" aria-label="WatchDog startup">
        <div className="splash-brand-row">
          <div className="splash-mark" aria-hidden="true">
            <span className="splash-eye" />
            <span className="splash-pulse" />
          </div>
          <div>
            <h1>WatchDog</h1>
            <p>{copy.subtitle}</p>
          </div>
        </div>

        <div className="splash-status" aria-live="polite">
          <span>{label}</span>
          <strong>{Math.round(progress.percent)}%</strong>
        </div>
        <div
          className="splash-progress-track"
          role="progressbar"
          aria-label={label}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(progress.percent)}
        >
          <div className="splash-progress-fill" style={{ width: `${progress.percent}%` }} />
        </div>
        <div className="splash-detail">{detail ?? "\u00a0"}</div>
      </section>
    </main>
  );
}
