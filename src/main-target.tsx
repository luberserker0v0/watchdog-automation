import "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import CombinedPage from "./modules/sample_target/CombinedPage";
import "./modules/sample_target/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <CombinedPage />
  </React.StrictMode>,
);
