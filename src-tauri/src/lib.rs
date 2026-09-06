pub mod config;
pub mod convert;
pub mod discovery;
pub mod file_kind;
pub mod ghostscript;
pub mod printer;
pub mod raster;
pub mod reachability;

use file_kind::FileKind;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Serialize)]
struct PrinterStatus {
    ip: String,
    name: String,
}

#[derive(Clone, Serialize)]
struct PrintProgress {
    stage: String,
}

fn emit_progress(app_handle: &tauri::AppHandle, stage: &str) {
    let _ = app_handle.emit(
        "print-progress",
        PrintProgress {
            stage: stage.to_string(),
        },
    );
}

fn config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Impossible de localiser le dossier de configuration : {e}"))?;
    Ok(dir.join("printer_config.json"))
}

fn resolve_printer_uri(app_handle: &tauri::AppHandle) -> Result<String, String> {
    let path = config_path(app_handle)?;

    if let Some(cached) = config::load(&path) {
        if reachability::is_reachable(&cached.ip, 631, Duration::from_millis(800)) {
            return Ok(printer::printer_uri(&cached.ip));
        }
    }

    let found = discovery::discover(Duration::from_secs(5))
        .map_err(|e| format!("Échec de la découverte réseau : {e}"))?
        .ok_or_else(|| {
            "Imprimante introuvable sur le réseau. Saisissez son IP manuellement.".to_string()
        })?;

    config::save(
        &path,
        &config::PrinterConfig {
            ip: found.ip.clone(),
            name: found.name,
        },
    )
    .map_err(|e| format!("Impossible d'enregistrer la configuration : {e}"))?;

    Ok(printer::printer_uri(&found.ip))
}

#[tauri::command]
fn discover_printer(app_handle: tauri::AppHandle) -> Result<String, String> {
    resolve_printer_uri(&app_handle)
}

#[tauri::command]
fn get_status(app_handle: tauri::AppHandle) -> Result<Option<PrinterStatus>, String> {
    let path = config_path(&app_handle)?;
    Ok(config::load(&path).map(|c| PrinterStatus {
        ip: c.ip,
        name: c.name,
    }))
}

#[tauri::command]
fn set_manual_ip(app_handle: tauri::AppHandle, ip: String) -> Result<(), String> {
    let path = config_path(&app_handle)?;
    config::save(
        &path,
        &config::PrinterConfig {
            ip,
            name: "Imprimante (IP manuelle)".to_string(),
        },
    )
    .map_err(|e| format!("Impossible d'enregistrer la configuration : {e}"))
}

#[tauri::command]
async fn print_file(app_handle: tauri::AppHandle, path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || print_file_blocking(&app_handle, &path))
        .await
        .map_err(|e| format!("La tâche d'impression a été interrompue : {e}"))?
}

fn print_file_blocking(app_handle: &tauri::AppHandle, path: &str) -> Result<String, String> {
    let input = PathBuf::from(path);
    let kind = file_kind::detect(&input);

    let pdf_path = match kind {
        FileKind::Pdf => input.clone(),
        FileKind::Office => {
            emit_progress(app_handle, "Conversion du document en PDF...");
            let soffice = convert::find_soffice_binary().ok_or_else(|| {
                "LibreOffice n'est pas installé. Installez-le pour convertir les fichiers Word/Excel.".to_string()
            })?;
            let out_dir = std::env::temp_dir().join("driver-imprimante");
            convert::convert_to_pdf(&soffice, &input, &out_dir)?
        }
        FileKind::Unsupported => {
            return Err(
                "Type de fichier non supporté. Formats acceptés : PDF, DOCX, XLSX, PPTX."
                    .to_string(),
            );
        }
    };

    emit_progress(app_handle, "Recherche de l'imprimante sur le réseau...");
    let uri = resolve_printer_uri(app_handle)?;

    let reasons = printer::get_state_reasons(&uri)?;
    if !reasons.is_empty() {
        return Err(format!(
            "L'imprimante signale un problème : {}",
            reasons.join(", ")
        ));
    }

    emit_progress(app_handle, "Préparation de la page pour l'impression...");
    let gs = ghostscript::find_ghostscript_binary().ok_or_else(|| {
        "Ghostscript n'est pas installé. Installez-le pour préparer les documents à imprimer."
            .to_string()
    })?;
    let raster_dir = std::env::temp_dir().join("driver-imprimante-raster");
    let ppm_pages = ghostscript::render_pdf_to_ppms(&gs, &pdf_path, &raster_dir)?;

    emit_progress(
        app_handle,
        &format!(
            "Encodage de {} page{}...",
            ppm_pages.len(),
            if ppm_pages.len() > 1 { "s" } else { "" }
        ),
    );
    let pages = ppm_pages
        .iter()
        .map(|p| ghostscript::parse_ppm(p))
        .collect::<Result<Vec<_>, _>>()?;
    let raster_bytes = raster::encode_document(&pages);

    emit_progress(
        app_handle,
        "Envoi à l'imprimante... cela peut prendre jusqu'à une minute selon l'imprimante.",
    );
    printer::send_print_job(&uri, raster_bytes)?;

    Ok("Document envoyé à l'imprimante.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            discover_printer,
            set_manual_ip,
            print_file,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
