import React from "react";
import ReactDOM from "react-dom/client";
import CombinedPage from "./modules/pipeline/components/target/CombinedPage";
import "./modules/pipeline/styles/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <CombinedPage />
  </React.StrictMode>,
);
