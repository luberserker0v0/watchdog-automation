import { initializeStandaloneI18n } from "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import SettingsPage from "./modules/sample_target/SettingsPage";
import "./modules/sample_target/target-ui.css";

void initializeStandaloneI18n().then(() => ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><SettingsPage /></React.StrictMode>,
));
