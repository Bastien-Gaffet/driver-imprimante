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
