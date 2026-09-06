use tauri_app_lib::{ghostscript, printer, raster};
use std::path::Path;

fn main() {
    let pdf_path = std::env::args().nth(1).expect("usage: test_print <pdf> <printer-ip>");
    let ip = std::env::args().nth(2).expect("usage: test_print <pdf> <printer-ip>");

    let gs = ghostscript::find_ghostscript_binary().expect("Ghostscript not found");
    println!("Using Ghostscript at {gs:?}");

    let out_dir = std::env::temp_dir().join("driver-imprimante-test");
    let ppm_pages = ghostscript::render_pdf_to_ppms(&gs, Path::new(&pdf_path), &out_dir)
        .expect("failed to render PDF to PPM");
    println!("Rendered {} page(s): {:?}", ppm_pages.len(), ppm_pages);

    let pages = ppm_pages
        .iter()
        .map(|p| ghostscript::parse_ppm(p))
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to parse PPM");

    let raster_bytes = raster::encode_document(&pages);
    println!("Encoded {} bytes of PWG-Raster", raster_bytes.len());

    let uri = printer::printer_uri(&ip);
    println!("Sending to {uri}...");
    printer::send_print_job(&uri, raster_bytes).expect("print job failed");
    println!("Print job sent successfully!");
}
