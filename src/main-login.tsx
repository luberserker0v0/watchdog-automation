import { initializeStandaloneI18n } from "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import LoginPage from "./modules/sample_target/LoginPage";
import "./modules/sample_target/target-ui.css";

void initializeStandaloneI18n().then(() => ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><LoginPage /></React.StrictMode>,
));
