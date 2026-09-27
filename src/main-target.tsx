import { initializeStandaloneI18n } from "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import CombinedPage from "./modules/sample_target/CombinedPage";
import "./modules/sample_target/target-ui.css";

void initializeStandaloneI18n().then(() => ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><CombinedPage /></React.StrictMode>,
));
