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
