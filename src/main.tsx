import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "highlight.js/styles/github-dark.css";
import "./styles/tokens.css";
import "./styles/globals.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Elemento #root não encontrado");
}

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
