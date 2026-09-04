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
