import React from "react";
import ReactDOM from "react-dom/client";
import { Theme } from "@radix-ui/themes"

import "@radix-ui/themes/styles.css";
import "./styles/view.css"

import App from "./app/view";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <Theme appearance="dark">
    <React.StrictMode>
      <App />
    </React.StrictMode>
  </Theme>
);
