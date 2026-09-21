import React from "react";
import ReactDOM from "react-dom/client";
import SettingsPage from "./modules/sample_target/SettingsPage";
import "./modules/sample_target/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SettingsPage />
  </React.StrictMode>,
);
