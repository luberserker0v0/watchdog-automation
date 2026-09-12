import React from "react";
import ReactDOM from "react-dom/client";
import DashboardPage from "./modules/pipeline/components/target/DashboardPage";
import "./modules/pipeline/styles/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <DashboardPage />
  </React.StrictMode>,
);
