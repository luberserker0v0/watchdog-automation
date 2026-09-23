import "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import DashboardPage from "./modules/sample_target/DashboardPage";
import "./modules/sample_target/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <DashboardPage />
  </React.StrictMode>,
);
