import { getCurrentWebview } from "@tauri-apps/api/webview";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface PrinterStatus {
  ip: string;
  name: string;
}

interface PrintProgress {
  stage: string;
}

const statusEl = document.querySelector<HTMLParagraphElement>("#status")!;
const currentPrinterEl = document.querySelector<HTMLParagraphElement>("#current-printer")!;
const manualIpForm = document.querySelector<HTMLFormElement>("#manual-ip-form")!;
const manualIpInput = document.querySelector<HTMLInputElement>("#manual-ip-input")!;
const dropzoneEl = document.querySelector<HTMLDivElement>("#dropzone")!;

function setStatus(message: string, isError: boolean, inProgress = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
  dropzoneEl.classList.toggle("busy", inProgress);
  manualIpForm.hidden = !isError;
}

listen<PrintProgress>("print-progress", (event) => {
  setStatus(event.payload.stage, false, true);
});

async function printDroppedFile(path: string) {
  setStatus(`Envoi de ${path}...`, false, true);
  try {
    const result = await invoke<string>("print_file", { path });
    setStatus(result, false, false);
  } catch (err) {
    setStatus(String(err), true, false);
  }
}

manualIpForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  try {
    await invoke("set_manual_ip", { ip: manualIpInput.value });
    setStatus("IP enregistrée. Réessayez l'impression.", false);
  } catch (err) {
    setStatus(String(err), true);
  }
});

getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type === "drop") {
    for (const path of event.payload.paths) {
      printDroppedFile(path);
    }
  }
});

async function showCurrentPrinter() {
  try {
    const status = await invoke<PrinterStatus | null>("get_status");
    currentPrinterEl.textContent = status ? `Imprimante : ${status.name} (${status.ip})` : "";
  } catch {
    currentPrinterEl.textContent = "";
  }
}

showCurrentPrinter();
