import React from "react";
import ReactDOM from "react-dom/client";
import LoginPage from "./modules/pipeline/components/target/LoginPage";
import "./modules/pipeline/styles/target-ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LoginPage />
  </React.StrictMode>,
);
