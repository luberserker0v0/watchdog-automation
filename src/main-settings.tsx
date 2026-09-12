import React from "react";
import ReactDOM from "react-dom/client";
import SettingsPage from "./modules/pipeline/components/target/SettingsPage";
import "./modules/pipeline/styles/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SettingsPage />
  </React.StrictMode>,
);
