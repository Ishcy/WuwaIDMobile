import { invoke } from "@tauri-apps/api/core";

const form = document.querySelector<HTMLFormElement>("#greet-form");
const input = document.querySelector<HTMLInputElement>("#greet-input");
const message = document.querySelector<HTMLElement>("#greet-msg");

if (!form || !input || !message) {
  throw new Error("Greeting form elements are missing.");
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();

  const name = input.value.trim();
  if (name.length === 0) {
    return;
  }

  message.textContent = await invoke<string>("greet", { name });
});
