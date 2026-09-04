use ipp::prelude::*;
use std::io::Cursor;

pub fn printer_uri(ip: &str) -> String {
    format!("ipp://{ip}:631/ipp/print")
}

pub fn parse_state_reasons(raw: &str) -> Vec<String> {
    raw.trim_matches(|c| c == '[' || c == ']')
        .split(',')
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
        .document_format("application/pdf")
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
    fn parses_bracketed_multi_value_reasons() {
        assert_eq!(
            parse_state_reasons("[media-empty, toner-low]"),
            vec!["media-empty".to_string(), "toner-low".to_string()]
        );
    }

    #[test]
    fn treats_none_as_no_reasons() {
        assert_eq!(parse_state_reasons("none"), Vec::<String>::new());
        assert_eq!(parse_state_reasons("[none]"), Vec::<String>::new());
    }
}
