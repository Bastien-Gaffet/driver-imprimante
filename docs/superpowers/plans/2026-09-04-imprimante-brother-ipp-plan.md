# Application d'impression directe (Tauri/IPP) — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri v2 desktop app that lets the user drag a PDF or Office
file onto a window and have it print directly on the Brother HL-L3240CDW (or
similar) network printer via IPP, bypassing the Windows print spooler
entirely, with the printer located automatically via mDNS.

**Architecture:** TypeScript/Vite frontend (drag-and-drop window, no
framework) talking to a Rust backend (`src-tauri/`) through Tauri commands.
The backend resolves the printer's address via mDNS (with a cached IP for
fast startup), converts Office documents to PDF via a LibreOffice
subprocess, and sends the PDF to the printer using the IPP protocol.

**Tech Stack:** Tauri v2, TypeScript, Vite, Rust, the `ipp` crate (v7,
`client` feature) for IPP, the `mdns-sd` crate (v0.21) for printer discovery,
LibreOffice (external binary, invoked via `soffice --headless
--convert-to pdf`) for Office-to-PDF conversion.

Full context and rationale: see
`docs/superpowers/specs/2026-09-04-imprimante-brother-ipp-design.md`.

---

## Reference: verified library APIs

These snippets were verified against real crate source during research and
are the contract every task below builds on. Re-check `cargo doc --open` if
anything doesn't compile as written — crate versions can drift.

- `ipp` crate, `Cargo.toml`: `ipp = { version = "7", default-features = false, features = ["client"] }`
- `mdns-sd` crate: `mdns-sd = "0.21"`
- `ServiceInfo::get_addresses_v4() -> HashSet<&Ipv4Addr>`, `get_fullname() -> &str`
- `IppClient::new(uri: Uri) -> IppClient`, `client.send(operation) -> Result<IppRequestResponse, IppError>`
- `response.header().status_code().is_success() -> bool`
- Tauri v2 app data dir: `app_handle.path().app_data_dir() -> tauri::Result<PathBuf>` (needs `use tauri::Manager;`)
- Tauri v2 drag-and-drop: `getCurrentWebview().onDragDropEvent(callback)` from `@tauri-apps/api/webview`

---

### Task 1: Scaffold the Tauri v2 project

**Files:**
- Create: entire Tauri v2 project skeleton at the repo root (`package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/`, `src-tauri/`)
- Modify: `src-tauri/Cargo.toml` (add `ipp`, `mdns-sd`, `tempfile` dev-dependency)

- [ ] **Step 1: Run the Tauri scaffolding CLI in the repo root**

Run:
```bash
cd "c:/Users/basti/Documents/Driver imprimante"
npm create tauri-app@latest . -- --manager npm --template vanilla-ts --identifier com.bastien.driverimprimante --force --yes
```
Expected: the command finishes without error and creates `package.json`,
`vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.ts`, `src/style.css`,
`src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`,
`src-tauri/tauri.conf.json` alongside the existing `docs/` and `.git/`.

- [ ] **Step 2: Fix the generated package name if it was sanitized incorrectly**

Open `package.json` and `src-tauri/Cargo.toml`. If either `"name"` /
`name = "..."` field is not `driver-imprimante` (npm/Cargo package names must
be lowercase with hyphens, no spaces), set it to `driver-imprimante` in both
files. Leave the Rust lib name (`driver_imprimante_lib` or similar, as
generated) untouched — just confirm what the scaffold produced, since later
tasks reference it in `src-tauri/src/main.rs`.

- [ ] **Step 3: Install JS dependencies**

Run:
```bash
npm install
```
Expected: exits 0, creates `node_modules/` and `package-lock.json`.

- [ ] **Step 4: Add the `ipp` and `mdns-sd` crates, plus `tempfile` for tests**

Edit `src-tauri/Cargo.toml`, add to the `[dependencies]` section:
```toml
ipp = { version = "7", default-features = false, features = ["client"] }
mdns-sd = "0.21"
```
Add a new section at the end of the file:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Verify the project builds**

Run:
```bash
cd src-tauri
cargo build
cd ..
```
Expected: exits 0 (first build will take a while — it compiles all
dependencies including `tauri`, `ipp`, `mdns-sd`).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
Scaffold Tauri v2 app and add ipp/mdns-sd dependencies

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: File-kind detection

**Files:**
- Create: `src-tauri/src/file_kind.rs`
- Modify: `src-tauri/src/lib.rs:1` (add `mod file_kind;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/file_kind.rs`:
```rust
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum FileKind {
    Pdf,
    Office,
    Unsupported,
}

pub fn detect(path: &Path) -> FileKind {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        Some(ext) if ext == "pdf" => FileKind::Pdf,
        Some(ext) if ["docx", "xlsx", "pptx"].contains(&ext.as_str()) => FileKind::Office,
        _ => FileKind::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_pdf() {
        assert_eq!(detect(&PathBuf::from("facture.pdf")), FileKind::Pdf);
    }

    #[test]
    fn detects_office_docx() {
        assert_eq!(detect(&PathBuf::from("lettre.docx")), FileKind::Office);
    }

    #[test]
    fn detects_office_xlsx_case_insensitive() {
        assert_eq!(detect(&PathBuf::from("budget.XLSX")), FileKind::Office);
    }

    #[test]
    fn rejects_unsupported_extension() {
        assert_eq!(detect(&PathBuf::from("photo.png")), FileKind::Unsupported);
    }

    #[test]
    fn rejects_missing_extension() {
        assert_eq!(detect(&PathBuf::from("README")), FileKind::Unsupported);
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add near the top (above any existing `use` or
function):
```rust
mod file_kind;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test file_kind
cd ..
```
Expected: `5 passed; 0 failed` for the `file_kind::tests` module.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/file_kind.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add file-kind detection (PDF vs Office vs unsupported)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Printer config cache

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod config;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/config.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PrinterConfig {
    pub ip: String,
    pub name: String,
}

pub fn load(config_path: &Path) -> Option<PrinterConfig> {
    let data = fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save(config_path: &Path, config: &PrinterConfig) -> std::io::Result<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(config_path, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("printer_config.json");
        let config = PrinterConfig {
            ip: "192.168.1.50".into(),
            name: "Brother HL-L3240CDW".into(),
        };

        save(&path, &config).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn load_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");

        assert_eq!(load(&path), None);
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add:
```rust
mod config;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test config::
cd ..
```
Expected: `2 passed; 0 failed`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add printer config cache (load/save as JSON)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Reachability check

**Files:**
- Create: `src-tauri/src/reachability.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod reachability;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/reachability.rs`:
```rust
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn is_reachable(ip: &str, port: u16, timeout: Duration) -> bool {
    let addr = match format!("{ip}:{port}").to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, timeout).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn reachable_when_port_open() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(is_reachable("127.0.0.1", port, Duration::from_millis(500)));
    }

    #[test]
    fn unreachable_when_port_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        assert!(!is_reachable("127.0.0.1", port, Duration::from_millis(200)));
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add:
```rust
mod reachability;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test reachability::
cd ..
```
Expected: `2 passed; 0 failed`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/reachability.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add TCP reachability check for cached printer IP

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: mDNS discovery

**Files:**
- Create: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod discovery;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/discovery.rs`:
```rust
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub struct DiscoveredPrinter {
    pub ip: String,
    pub name: String,
}

pub fn printer_from_service_info(info: &ServiceInfo) -> Option<DiscoveredPrinter> {
    let ip = info.get_addresses_v4().into_iter().next()?.to_string();
    let fullname = info.get_fullname();
    let name = fullname.split("._").next().unwrap_or(fullname).to_string();
    Some(DiscoveredPrinter { ip, name })
}

pub fn discover(timeout: Duration) -> mdns_sd::Result<Option<DiscoveredPrinter>> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse("_ipp._tcp.local.")?;
    let deadline = Instant::now() + timeout;

    let found = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break None;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(printer) = printer_from_service_info(&info) {
                    break Some(printer);
                }
            }
            Ok(_) => continue,
            Err(_) => break None,
        }
    };

    mdns.shutdown()?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ip_and_name_from_service_info() {
        let info = ServiceInfo::new(
            "_ipp._tcp.local.",
            "Brother HL-L3240CDW",
            "brother-printer.local.",
            "192.168.1.50",
            631u16,
            None,
        )
        .unwrap();

        let printer = printer_from_service_info(&info).unwrap();

        assert_eq!(printer.ip, "192.168.1.50");
        assert_eq!(printer.name, "Brother HL-L3240CDW");
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add:
```rust
mod discovery;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test discovery::
cd ..
```
Expected: `1 passed; 0 failed`.

Note: `discover()` itself (the live mDNS browse) is not unit-tested — it
depends on real network multicast traffic. It is exercised manually in
Task 9 against the real printer.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/discovery.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add mDNS printer discovery

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: IPP client (print job + state check)

**Files:**
- Create: `src-tauri/src/printer.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod printer;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/printer.rs`:
```rust
use ipp::prelude::*;
use std::io::Cursor;

pub fn printer_uri(ip: &str) -> String {
    format!("ipp://{ip}:631/ipp/print")
}

pub fn parse_state_reasons(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "none")
        .collect()
}

pub fn send_print_job(uri_str: &str, pdf_bytes: Vec<u8>) -> Result<(), String> {
    let uri: Uri = uri_str
        .parse()
        .map_err(|e| format!("URI imprimante invalide : {e}"))?;
    let payload = IppPayload::new(Cursor::new(pdf_bytes));
    let operation = IppOperationBuilder::print_job(uri.clone(), payload)
        .user_name("driver-imprimante")
        .job_title("Impression depuis Driver imprimante")
        .build()
        .map_err(|e| format!("Échec de préparation du job IPP : {e}"))?;

    let client = IppClient::new(uri);
    let response = client
        .send(operation)
        .map_err(|e| format!("Échec de communication avec l'imprimante : {e}"))?;

    if response.header().status_code().is_success() {
        Ok(())
    } else {
        Err(format!(
            "L'imprimante a refusé le job (code {})",
            response.header().status_code()
        ))
    }
}

pub fn get_state_reasons(uri_str: &str) -> Result<Vec<String>, String> {
    let uri: Uri = uri_str
        .parse()
        .map_err(|e| format!("URI imprimante invalide : {e}"))?;
    let operation = IppOperationBuilder::get_printer_attributes(uri.clone())
        .build()
        .map_err(|e| format!("Échec de préparation de la requête d'état : {e}"))?;
    let client = IppClient::new(uri);
    let response = client
        .send(operation)
        .map_err(|e| format!("Échec de communication avec l'imprimante : {e}"))?;

    let raw = response
        .attributes()
        .groups_of(DelimiterTag::PrinterAttributes)
        .next()
        .and_then(|group| group.get("printer-state-reasons"))
        .map(|attr| attr.value().to_string())
        .unwrap_or_default();

    Ok(parse_state_reasons(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_uri() {
        assert_eq!(
            printer_uri("192.168.1.50"),
            "ipp://192.168.1.50:631/ipp/print"
        );
    }

    #[test]
    fn parses_multiple_reasons() {
        assert_eq!(
            parse_state_reasons("media-empty,toner-low"),
            vec!["media-empty".to_string(), "toner-low".to_string()]
        );
    }

    #[test]
    fn treats_none_as_no_reasons() {
        assert_eq!(parse_state_reasons("none"), Vec::<String>::new());
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add:
```rust
mod printer;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test printer::
cd ..
```
Expected: `3 passed; 0 failed`.

Note: `send_print_job` and `get_state_reasons` themselves need a real
reachable printer and are not unit-tested; they are exercised manually in
Task 9.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/printer.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add IPP client: print job submission and state check

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: LibreOffice conversion

**Files:**
- Create: `src-tauri/src/convert.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod convert;`)

- [ ] **Step 1: Write the module with its tests**

Create `src-tauri/src/convert.rs`:
```rust
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn soffice_args(input: &Path, out_dir: &Path) -> Vec<String> {
    vec![
        "--headless".to_string(),
        "--convert-to".to_string(),
        "pdf".to_string(),
        "--outdir".to_string(),
        out_dir.to_string_lossy().into_owned(),
        input.to_string_lossy().into_owned(),
    ]
}

pub fn find_soffice_binary_in(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

pub fn find_soffice_binary() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
        PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
    ];
    find_soffice_binary_in(&candidates)
}

pub fn convert_to_pdf(soffice_path: &Path, input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Impossible de créer le dossier temporaire : {e}"))?;

    let status = Command::new(soffice_path)
        .args(soffice_args(input, out_dir))
        .status()
        .map_err(|e| format!("Échec de lancement de LibreOffice : {e}"))?;

    if !status.success() {
        return Err(format!("LibreOffice a échoué (code {status})"));
    }

    let stem = input.file_stem().ok_or("Nom de fichier invalide")?;
    let output = out_dir.join(stem).with_extension("pdf");
    if output.exists() {
        Ok(output)
    } else {
        Err("LibreOffice n'a pas produit de fichier PDF".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_soffice_args() {
        let input = PathBuf::from(r"C:\docs\lettre.docx");
        let out_dir = PathBuf::from(r"C:\temp\driver-imprimante");

        let args = soffice_args(&input, &out_dir);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        assert_eq!(
            args,
            vec![
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                r"C:\temp\driver-imprimante",
                r"C:\docs\lettre.docx",
            ]
        );
    }

    #[test]
    fn finds_first_existing_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does_not_exist.exe");
        let existing = dir.path().join("soffice.exe");
        std::fs::write(&existing, b"").unwrap();

        let found = find_soffice_binary_in(&[missing, existing.clone()]);

        assert_eq!(found, Some(existing));
    }

    #[test]
    fn returns_none_when_nothing_exists() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does_not_exist.exe");

        assert_eq!(find_soffice_binary_in(&[missing]), None);
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add:
```rust
mod convert;
```

- [ ] **Step 3: Run the tests**

Run:
```bash
cd src-tauri
cargo test convert::
cd ..
```
Expected: `3 passed; 0 failed`.

Note: `convert_to_pdf` itself needs a real LibreOffice install and is not
unit-tested; it is exercised manually in Task 9.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/convert.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Add LibreOffice-based Office-to-PDF conversion

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Wire the Tauri commands

**Files:**
- Modify: `src-tauri/src/lib.rs` (full rewrite of the generated content)

- [ ] **Step 1: Replace the contents of `src-tauri/src/lib.rs`**

The scaffold generated a `lib.rs` with a placeholder `greet` command and a
`run()` function. Replace the whole file with:
```rust
mod config;
mod convert;
mod discovery;
mod file_kind;
mod printer;
mod reachability;

use file_kind::FileKind;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

#[derive(Serialize)]
struct PrinterStatus {
    ip: String,
    name: String,
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
fn print_file(app_handle: tauri::AppHandle, path: String) -> Result<String, String> {
    let input = PathBuf::from(&path);
    let kind = file_kind::detect(&input);

    let pdf_path = match kind {
        FileKind::Pdf => input.clone(),
        FileKind::Office => {
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

    let uri = resolve_printer_uri(&app_handle)?;

    let reasons = printer::get_state_reasons(&uri)?;
    if !reasons.is_empty() {
        return Err(format!(
            "L'imprimante signale un problème : {}",
            reasons.join(", ")
        ));
    }

    let bytes = std::fs::read(&pdf_path).map_err(|e| format!("Impossible de lire le fichier : {e}"))?;
    printer::send_print_job(&uri, bytes)?;

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
```

- [ ] **Step 2: Run all backend tests to confirm nothing broke**

Run:
```bash
cd src-tauri
cargo test
cd ..
```
Expected: all tests from Tasks 2-7 still pass (`ok` for every module,
`file_kind`, `config`, `reachability`, `discovery`, `printer`, `convert`).

- [ ] **Step 3: Build the backend**

Run:
```bash
cd src-tauri
cargo build
cd ..
```
Expected: exits 0. If it fails on `tauri_plugin_opener::init()` because the
scaffold didn't wire that plugin the same way, open `src-tauri/src/main.rs`
and confirm it calls `{crate_name}_lib::run();` — that file is generated by
the scaffold and should not need changes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Wire discover_printer, set_manual_ip, print_file and get_status commands

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Frontend — drag-and-drop UI

**Files:**
- Modify: `index.html` (replace generated content)
- Modify: `src/main.ts` (replace generated content)
- Modify: `src/style.css` (replace generated content)

- [ ] **Step 1: Replace `index.html`**

```html
<!doctype html>
<html lang="fr">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Driver imprimante</title>
    <script type="module" src="/src/main.ts" defer></script>
    <link rel="stylesheet" href="/src/style.css" />
  </head>
  <body>
    <main class="app">
      <h1>Driver imprimante</h1>
      <div id="dropzone">
        <p id="status">Glissez un fichier PDF ou Word/Excel sur cette fenêtre pour l'imprimer.</p>
        <p id="current-printer"></p>
      </div>
      <form id="manual-ip-form" hidden>
        <label for="manual-ip-input">IP de l'imprimante</label>
        <input id="manual-ip-input" type="text" placeholder="192.168.1.50" required />
        <button type="submit">Enregistrer</button>
      </form>
    </main>
  </body>
</html>
```

- [ ] **Step 2: Replace `src/main.ts`**

```typescript
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { invoke } from "@tauri-apps/api/core";

interface PrinterStatus {
  ip: string;
  name: string;
}

const statusEl = document.querySelector<HTMLParagraphElement>("#status")!;
const currentPrinterEl = document.querySelector<HTMLParagraphElement>("#current-printer")!;
const manualIpForm = document.querySelector<HTMLFormElement>("#manual-ip-form")!;
const manualIpInput = document.querySelector<HTMLInputElement>("#manual-ip-input")!;

function setStatus(message: string, isError: boolean) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
  manualIpForm.hidden = !isError;
}

async function printDroppedFile(path: string) {
  setStatus(`Envoi de ${path}...`, false);
  try {
    const result = await invoke<string>("print_file", { path });
    setStatus(result, false);
  } catch (err) {
    setStatus(String(err), true);
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
```

- [ ] **Step 3: Replace `src/style.css`**

```css
:root {
  color-scheme: light dark;
  font-family: system-ui, sans-serif;
}

body {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
}

.app {
  text-align: center;
  padding: 2rem;
}

#dropzone {
  border: 2px dashed #888;
  border-radius: 12px;
  padding: 3rem 2rem;
  min-width: 320px;
}

#status.error {
  color: #d92626;
  font-weight: 600;
}

#manual-ip-form {
  margin-top: 1rem;
  display: flex;
  gap: 0.5rem;
  justify-content: center;
}
```

- [ ] **Step 4: Commit**

```bash
git add index.html src/main.ts src/style.css
git commit -m "$(cat <<'EOF'
Add drag-and-drop UI wired to the print_file command

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: End-to-end manual verification

**Files:** none (manual verification against real hardware)

- [ ] **Step 1: Confirm LibreOffice is installed**

Run:
```bash
"C:\Program Files\LibreOffice\program\soffice.exe" --version
```
Expected: prints a LibreOffice version string. If this fails, install
LibreOffice before continuing (free download from libreoffice.org), since
Task 7's `find_soffice_binary()` looks for this exact path.

- [ ] **Step 2: Run the app in dev mode**

Run:
```bash
npm run tauri dev
```
Expected: a window titled "Driver imprimante" opens showing the
drag-and-drop prompt.

- [ ] **Step 3: Print a real PDF**

Drag a real PDF file from Windows Explorer onto the app window. Expected:
status changes to "Envoi de ...", then to "Document envoyé à l'imprimante."
and the physical printer produces the page.

- [ ] **Step 4: Print a real Word document**

Drag a real `.docx` file onto the app window. Expected: same success
message, and the printed page matches the document's content (verifies the
LibreOffice conversion step).

- [ ] **Step 5: Verify the manual-IP fallback**

Temporarily disconnect the printer from the network (or unplug it), delete
the cached config file at the path printed by `app_handle.path().app_data_dir()`
(typically `%APPDATA%\com.bastien.driverimprimante\printer_config.json`),
then drop a PDF onto the app. Expected: the status area shows the
"Imprimante introuvable" error and reveals the manual-IP form. Reconnect the
printer, enter its IP, submit, and drop the PDF again — it should print.

- [ ] **Step 6: Commit any fixes made during verification**

If any of the above steps required code changes, stage and commit them with
a descriptive message following the same `git commit -m "$(cat <<'EOF' ... EOF)"`
pattern used in earlier tasks, ending with the `Co-Authored-By` trailer.
