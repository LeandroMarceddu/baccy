import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

console.log("main.ts loaded");

let app;

try {
  app = mount(App, {
    target: document.getElementById("app")!,
  });
  console.log("App mounted successfully", app);
} catch (error) {
  console.error("Failed to mount app:", error);
  document.body.innerHTML = `<div style="padding: 20px; color: red; font-family: monospace;">
    <h1>Error mounting app</h1>
    <pre>${error}</pre>
  </div>`;
}

export default app;
