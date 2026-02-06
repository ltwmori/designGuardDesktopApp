import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { bootAnalytics } from "./lib/analytics";
import "./index.css";

// Restore analytics from previous consent decision (no-op if declined or undecided)
bootAnalytics();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
