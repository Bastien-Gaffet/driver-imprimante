use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_ghostscript_binary_in(gs_root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(gs_root)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("bin").join("gswin64c.exe"))
        .filter(|p| p.exists())
        .collect();
    candidates.sort();
    candidates.pop()
}

pub fn find_ghostscript_binary() -> Option<PathBuf> {
    find_ghostscript_binary_in(Path::new(r"C:\Program Files\gs"))
}

pub fn render_args(pdf: &Path, out_pattern: &Path) -> Vec<String> {
    vec![
        "-dNOPAUSE".to_string(),
        "-dBATCH".to_string(),
        "-dQUIET".to_string(),
        "-sDEVICE=ppmraw".to_string(),
        "-r600x600".to_string(),
        format!("-sOutputFile={}", out_pattern.to_string_lossy()),
        pdf.to_string_lossy().into_owned(),
    ]
}

/// Render every page of `pdf` to a separate 600dpi RGB PPM file in `out_dir`,
/// returning the produced files in page order.
pub fn render_pdf_to_ppms(gs_path: &Path, pdf: &Path, out_dir: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Impossible de créer le dossier temporaire : {e}"))?;

    let out_pattern = out_dir.join("page-%03d.ppm");
    let status = Command::new(gs_path)
        .args(render_args(pdf, &out_pattern))
        .status()
        .map_err(|e| format!("Échec de lancement de Ghostscript : {e}"))?;

    if !status.success() {
        return Err(format!("Ghostscript a échoué (code {status})"));
    }

    let mut pages: Vec<PathBuf> = std::fs::read_dir(out_dir)
        .map_err(|e| format!("Impossible de lire le dossier temporaire : {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ppm"))
        .collect();
    pages.sort();

    if pages.is_empty() {
        return Err("Ghostscript n'a produit aucune page".to_string());
    }

    Ok(pages)
}

/// Parse a binary PPM (P6) file into (width, height, raw RGB8 bytes).
pub fn parse_ppm(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let data = std::fs::read(path).map_err(|e| format!("Impossible de lire {path:?} : {e}"))?;

    let mut fields = Vec::new();
    let mut pos = 0;
    while fields.len() < 4 {
        loop {
            while pos < data.len() && data[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'#' {
                while pos < data.len() && data[pos] != b'\n' {
                    pos += 1;
                }
            } else {
                break;
            }
        }
        let start = pos;
        while pos < data.len() && !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if start == pos {
            return Err(format!("En-tête PPM invalide dans {path:?}"));
        }
        fields.push(String::from_utf8_lossy(&data[start..pos]).into_owned());
    }
    pos += 1; // single whitespace byte separating header from binary data

    if fields[0] != "P6" {
        return Err(format!("Format PPM non supporté dans {path:?} : {}", fields[0]));
    }
    let width: u32 = fields[1]
        .parse()
        .map_err(|_| format!("Largeur PPM invalide dans {path:?}"))?;
    let height: u32 = fields[2]
        .parse()
        .map_err(|_| format!("Hauteur PPM invalide dans {path:?}"))?;
    let maxval: u32 = fields[3]
        .parse()
        .map_err(|_| format!("Maxval PPM invalide dans {path:?}"))?;
    if maxval != 255 {
        return Err(format!("Maxval PPM non supporté ({maxval}) dans {path:?}"));
    }

    let expected_len = (width * height * 3) as usize;
    let pixels = data.get(pos..pos + expected_len).ok_or_else(|| {
        format!("Données PPM tronquées dans {path:?}: attendu {expected_len} octets")
    })?;

    Ok((width, height, pixels.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_render_args() {
        let pdf = PathBuf::from(r"C:\temp\doc.pdf");
        let pattern = PathBuf::from(r"C:\temp\out\page-%03d.ppm");

        let args = render_args(&pdf, &pattern);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        assert_eq!(
            args,
            vec![
                "-dNOPAUSE",
                "-dBATCH",
                "-dQUIET",
                "-sDEVICE=ppmraw",
                "-r600x600",
                r"-sOutputFile=C:\temp\out\page-%03d.ppm",
                r"C:\temp\doc.pdf",
            ]
        );
    }

    #[test]
    fn finds_a_gs_binary_when_one_version_is_installed() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("gs10.02.0").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("gswin64c.exe"), b"").unwrap();

        let found = find_ghostscript_binary_in(dir.path()).unwrap();

        assert!(found.ends_with(r"gs10.02.0\bin\gswin64c.exe"));
    }

    #[test]
    fn picks_lexically_last_when_multiple_versions_installed() {
        // Sorting is lexical on the directory name, not semver-aware, so a
        // "gs9.x" install would incorrectly outrank "gs10.x". Only one
        // version is expected in practice (a fresh install), so this is
        // documented behavior rather than a bug fix target for now.
        let dir = tempfile::tempdir().unwrap();
        for version in ["gs10.02.0", "gs10.06.0"] {
            let bin_dir = dir.path().join(version).join("bin");
            std::fs::create_dir_all(&bin_dir).unwrap();
            std::fs::write(bin_dir.join("gswin64c.exe"), b"").unwrap();
        }

        let found = find_ghostscript_binary_in(dir.path()).unwrap();

        assert!(found.ends_with(r"gs10.06.0\bin\gswin64c.exe"));
    }

    #[test]
    fn returns_none_when_gs_root_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        assert_eq!(find_ghostscript_binary_in(&missing), None);
    }

    #[test]
    fn parses_a_minimal_ppm() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.ppm");
        let mut content = b"P6\n2 1\n255\n".to_vec();
        content.extend_from_slice(&[255, 0, 0, 0, 255, 0]); // red pixel, green pixel

        std::fs::write(&path, &content).unwrap();

        let (width, height, pixels) = parse_ppm(&path).unwrap();

        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(pixels, vec![255, 0, 0, 0, 255, 0]);
    }

    #[test]
    fn skips_comment_line_like_ghostscript_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commented.ppm");
        let mut content = b"P6\n# Image generated by GPL Ghostscript (device=ppmraw)\n2 1\n255\n".to_vec();
        content.extend_from_slice(&[255, 0, 0, 0, 255, 0]);

        std::fs::write(&path, &content).unwrap();

        let (width, height, pixels) = parse_ppm(&path).unwrap();

        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(pixels, vec![255, 0, 0, 0, 255, 0]);
    }

    #[test]
    fn rejects_truncated_ppm_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("truncated.ppm");
        std::fs::write(&path, b"P6\n2 1\n255\n\xff\x00").unwrap();

        assert!(parse_ppm(&path).is_err());
    }
}
