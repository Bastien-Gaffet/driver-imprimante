const PWG_HEADER_SIZE: usize = 1796;
const CHUNK_BYTES: usize = 3; // sRGB, 8 bits per channel

fn write_u32_be(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn build_page_header(width: u32, height: u32, page_count: u32) -> [u8; PWG_HEADER_SIZE] {
    let mut header = [0u8; PWG_HEADER_SIZE];
    header[0..9].copy_from_slice(b"PwgRaster");
    write_u32_be(&mut header, 276, 600); // HWResolution.feed_res_dpi
    write_u32_be(&mut header, 280, 600); // HWResolution.cross_feed_res_dpi
    write_u32_be(&mut header, 340, 1); // NumCopies
    write_u32_be(&mut header, 352, width * 72 / 600); // PageSize.width (points)
    write_u32_be(&mut header, 356, height * 72 / 600); // PageSize.height (points)
    write_u32_be(&mut header, 372, width); // Width
    write_u32_be(&mut header, 376, height); // Height
    write_u32_be(&mut header, 384, 8); // BitsPerColor
    write_u32_be(&mut header, 388, 24); // BitsPerPixel
    write_u32_be(&mut header, 392, width * CHUNK_BYTES as u32); // BytesPerLine
    write_u32_be(&mut header, 400, 19); // ColorSpace = Srgb
    write_u32_be(&mut header, 420, 3); // NumColors
    write_u32_be(&mut header, 452, page_count); // TotalPageCount
    header
}

/// Encode one scanline (packbits-style, per PWG 5102.4 4.3): runs of >=2
/// identical pixel chunks use repeat-mode; everything else is literal.
fn encode_line(line: &[u8], out: &mut Vec<u8>) {
    let n = line.len() / CHUNK_BYTES;
    let chunk = |i: usize| &line[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES];

    let mut i = 0;
    while i < n {
        let mut run = 1;
        while run < 128 && i + run < n && chunk(i + run) == chunk(i) {
            run += 1;
        }

        if run >= 2 {
            out.push((run - 1) as u8);
            out.extend_from_slice(chunk(i));
            i += run;
        } else {
            let start = i;
            let mut len = 0;
            while len < 129 && i < n {
                if i + 1 < n && chunk(i) == chunk(i + 1) {
                    break;
                }
                len += 1;
                i += 1;
            }
            if len == 0 {
                out.push(0);
                out.extend_from_slice(chunk(i));
                i += 1;
            } else {
                out.push((257 - len) as u8);
                out.extend_from_slice(&line[start * CHUNK_BYTES..(start + len) * CHUNK_BYTES]);
            }
        }
    }
}

fn encode_page_pixels(width: u32, height: u32, rgb: &[u8], out: &mut Vec<u8>) {
    let bytes_per_line = width as usize * CHUNK_BYTES;
    for y in 0..height as usize {
        let line = &rgb[y * bytes_per_line..(y + 1) * bytes_per_line];
        out.push(0); // line-repeat count - 1 = 0 => each line emitted once
        encode_line(line, out);
    }
}

/// Encode a full PWG-Raster document (sync word + one page section per
/// (width, height, rgb8_pixels) entry) ready to send as an IPP payload
/// with document-format "image/pwg-raster".
pub fn encode_document(pages: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RaS2");
    let page_count = pages.len() as u32;
    for (width, height, rgb) in pages {
        out.extend_from_slice(&build_page_header(*width, *height, page_count));
        encode_page_pixels(*width, *height, rgb, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwgraster::PWGReader;
    use std::io::Cursor;

    fn decode_first_page(encoded: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut reader = PWGReader::new(Cursor::new(encoded), true).unwrap();

        reader
            .next_page(|page| {
                let width = page.header().Width().unwrap();
                let height = page.header().Height().unwrap();
                let mut pixels = Vec::new();
                page.unpack(&mut pixels)?;
                Ok((width, height, pixels))
            })
            .unwrap()
            .unwrap()
    }

    #[test]
    fn round_trips_a_small_image_with_runs_and_literals() {
        let width = 4;
        let height = 3;
        // Row 1: all-black (run). Row 2: four distinct colors (literal).
        // Row 3: two identical pairs (two runs).
        let rgb = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // row 1
            10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, // row 2
            1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, // row 3
        ];

        let encoded = encode_document(&[(width, height, rgb.clone())]);
        let (decoded_width, decoded_height, decoded_pixels) = decode_first_page(&encoded);

        assert_eq!(decoded_width, width);
        assert_eq!(decoded_height, height);
        assert_eq!(decoded_pixels, rgb);
    }

    #[test]
    fn round_trips_a_long_uniform_row() {
        let width = 300;
        let height = 1;
        let rgb = vec![255u8; (width * 3) as usize];

        let encoded = encode_document(&[(width, height, rgb.clone())]);
        let (decoded_width, decoded_height, decoded_pixels) = decode_first_page(&encoded);

        assert_eq!(decoded_width, width);
        assert_eq!(decoded_height, height);
        assert_eq!(decoded_pixels, rgb);
    }

    #[test]
    fn header_declares_srgb_and_600dpi() {
        let header = build_page_header(100, 50, 1);

        assert_eq!(&header[0..9], b"PwgRaster");
        assert_eq!(u32::from_be_bytes(header[276..280].try_into().unwrap()), 600);
        assert_eq!(u32::from_be_bytes(header[400..404].try_into().unwrap()), 19);
        assert_eq!(u32::from_be_bytes(header[384..388].try_into().unwrap()), 8);
    }
}
