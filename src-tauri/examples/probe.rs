use ipp::prelude::*;

fn main() {
    let ip = std::env::args()
        .nth(1)
        .expect("usage: cargo run --example probe -- <printer-ip>");
    let uri: Uri = format!("ipp://{ip}:631/ipp/print").parse().unwrap();
    let operation = IppOperationBuilder::get_printer_attributes(uri.clone())
        .build()
        .unwrap();
    let client = IppClient::new(uri);
    let response = client.send(operation).unwrap();

    println!("status: {}", response.header().status_code());

    for group in response.attributes().groups_of(DelimiterTag::PrinterAttributes) {
        for attr in group.attributes() {
            println!("{} = {}", attr.name(), attr.value());
        }
    }
}
