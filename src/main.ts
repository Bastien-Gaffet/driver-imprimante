import { getCurrentWebview } from "@tauri-apps/api/webview";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

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

const updateBanner = document.querySelector<HTMLDivElement>("#update-banner")!;
const updateText = document.querySelector<HTMLParagraphElement>("#update-text")!;
const updateInstallBtn = document.querySelector<HTMLButtonElement>("#update-install")!;

async function checkForUpdate() {
  let update: Update | null;
  try {
    update = await check();
  } catch {
    return;
  }
  if (!update) return;

  updateText.textContent = `Mise à jour ${update.version} disponible.`;
  updateBanner.hidden = false;

  updateInstallBtn.addEventListener(
    "click",
    async () => {
      updateInstallBtn.disabled = true;
      updateText.textContent = "Téléchargement de la mise à jour...";
      try {
        await update!.downloadAndInstall();
        updateText.textContent = "Redémarrage...";
        await relaunch();
      } catch (err) {
        updateText.textContent = `Échec de la mise à jour : ${String(err)}`;
        updateInstallBtn.disabled = false;
      }
    },
    { once: true },
  );
}

checkForUpdate();
