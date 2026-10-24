//! Texture sizes straight from the image headers (PNG, JPEG, KTX2, WebP),
//! so the analysis needs no image decoder, plus a tiny PNG writer for the
//! fixture models.

/// Width and height from an image's header, if it is a format we know.
pub(crate) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    png(bytes)
        .or_else(|| ktx2(bytes))
        .or_else(|| jpeg(bytes))
        .or_else(|| webp(bytes))
}

/// The MIME type a header says, for images without one in the JSON.
pub(crate) fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if png(bytes).is_some() {
        Some("image/png")
    } else if ktx2(bytes).is_some() {
        Some("image/ktx2")
    } else if jpeg(bytes).is_some() {
        Some("image/jpeg")
    } else if webp(bytes).is_some() {
        Some("image/webp")
    } else {
        None
    }
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn png(b: &[u8]) -> Option<(u32, u32)> {
    if b.get(..8)? != PNG_SIGNATURE || b.get(12..16)? != b"IHDR" {
        return None;
    }
    Some((be32(b, 16)?, be32(b, 20)?))
}

fn ktx2(b: &[u8]) -> Option<(u32, u32)> {
    const ID: [u8; 12] = [
        0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
    ];
    if b.get(..12)? != ID {
        return None;
    }
    Some((le32(b, 20)?, le32(b, 24)?.max(1)))
}

fn jpeg(b: &[u8]) -> Option<(u32, u32)> {
    if b.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut at = 2;
    while at + 4 <= b.len() {
        if b[at] != 0xFF {
            at += 1;
            continue;
        }
        let marker = b[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        if (0xD0..=0xD9).contains(&marker) || marker == 0x01 {
            at += 2;
            continue;
        }
        let len = usize::from(be16(b, at + 2)?);
        let sof = (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker);
        if sof {
            let h = be16(b, at + 5)?;
            let w = be16(b, at + 7)?;
            return Some((u32::from(w), u32::from(h)));
        }
        at += 2 + len;
    }
    None
}

fn webp(b: &[u8]) -> Option<(u32, u32)> {
    if b.get(..4)? != b"RIFF" || b.get(8..12)? != b"WEBP" {
        return None;
    }
    match b.get(12..16)? {
        b"VP8X" => {
            let w = u32::from_le_bytes([b[24], b[25], b[26], 0]) + 1;
            let h = u32::from_le_bytes([*b.get(27)?, *b.get(28)?, *b.get(29)?, 0]) + 1;
            Some((w, h))
        }
        b"VP8L" => {
            let bits = le32(b, 21)?;
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        b"VP8 " => {
            let w = u32::from(u16::from_le_bytes([*b.get(26)?, *b.get(27)?]) & 0x3FFF);
            let h = u32::from(u16::from_le_bytes([*b.get(28)?, *b.get(29)?]) & 0x3FFF);
            Some((w, h))
        }
        _ => None,
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encodes RGB pixels (row major, 3 bytes each) as a valid, uncompressed PNG.
pub(crate) fn write_png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let row = width as usize * 3;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * row..(y + 1) * row]);
    }
    // A zlib stream of stored deflate blocks.
    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65_535).peekable();
    if blocks.peek().is_none() {
        z.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        z.push(u8::from(blocks.peek().is_none()));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = PNG_SIGNATURE.to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sizes_from_headers() {
        let png = write_png(4, 2, &[200; 24]);
        assert_eq!(dimensions(&png), Some((4, 2)));
        assert_eq!(sniff(&png), Some("image/png"));
        // The well known CRC of "IEND".
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);

        let mut ktx = vec![
            0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
        ];
        ktx.extend_from_slice(&[0; 8]);
        ktx.extend_from_slice(&2048u32.to_le_bytes());
        ktx.extend_from_slice(&1024u32.to_le_bytes());
        assert_eq!(dimensions(&ktx), Some((2048, 1024)));

        // SOI, an APP0 segment, then SOF0 with height 300 and width 500.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01,
            0x2C, 0x01, 0xF4, 0x03,
        ];
        assert_eq!(dimensions(&jpeg), Some((500, 300)));
        assert_eq!(dimensions(b"not an image"), None);
    }
}
