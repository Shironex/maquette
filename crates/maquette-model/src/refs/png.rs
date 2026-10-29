//! Just enough PNG decoding for the ref checks: 8 bit grey, grey with
//! alpha, RGB, RGBA and palette images, not interlaced, which is what
//! Codex, the Lab and most tools save. Anything else reads as `None` and
//! the pixel checks stand down.

use std::io::Read as _;

use flate2::read::ZlibDecoder;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// Bigger images are refused rather than inflated: a ref is a few
/// thousand pixels across.
const MAX_PIXELS: u64 = 8192 * 8192;

/// Decoded pixels, four bytes (RGBA) each, row major.
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl Pixels {
    pub fn at(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i16::from(a), i16::from(b), i16::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Undoes the per row filters in place; `bpp` is bytes per pixel.
fn unfilter(raw: &[u8], stride: usize, height: usize, bpp: usize) -> Option<Vec<u8>> {
    let mut out = vec![0u8; stride * height];
    for y in 0..height {
        let row = raw.get(y * (stride + 1)..(y + 1) * (stride + 1))?;
        let (filter, line) = (row[0], &row[1..]);
        for x in 0..stride {
            let a = if x >= bpp {
                out[y * stride + x - bpp]
            } else {
                0
            };
            let b = if y > 0 { out[(y - 1) * stride + x] } else { 0 };
            let c = if x >= bpp && y > 0 {
                out[(y - 1) * stride + x - bpp]
            } else {
                0
            };
            let add = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
                4 => paeth(a, b, c),
                _ => return None,
            };
            out[y * stride + x] = line[x].wrapping_add(add);
        }
    }
    Some(out)
}

/// The image's pixels, or `None` for a file this decoder does not read.
pub fn decode(bytes: &[u8]) -> Option<Pixels> {
    if bytes.get(..8)? != SIGNATURE {
        return None;
    }
    let (mut at, mut header, mut palette, mut idat) = (8, None, Vec::new(), Vec::new());
    while at + 8 <= bytes.len() {
        let len = be32(bytes, at)? as usize;
        let kind = bytes.get(at + 4..at + 8)?;
        let data = bytes.get(at + 8..at + 8 + len)?;
        match kind {
            b"IHDR" => header = Some(data.to_vec()),
            b"PLTE" => palette = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let header = header?;
    let (width, height) = (be32(&header, 0)?, be32(&header, 4)?);
    let (depth, color, interlace) = (*header.get(8)?, *header.get(9)?, *header.get(12)?);
    if depth != 8 || interlace != 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }
    let channels = match color {
        0 | 3 => 1,
        4 => 2,
        2 => 3,
        6 => 4,
        _ => return None,
    };
    let (width, height) = (width as usize, height as usize);
    let stride = width * channels;
    let mut raw = Vec::with_capacity((stride + 1) * height);
    ZlibDecoder::new(idat.as_slice())
        .read_to_end(&mut raw)
        .ok()?;
    let plain = unfilter(&raw, stride, height, channels)?;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for px in plain.chunks_exact(channels) {
        let pixel = match color {
            0 => [px[0], px[0], px[0], 255],
            4 => [px[0], px[0], px[0], px[1]],
            2 => [px[0], px[1], px[2], 255],
            6 => [px[0], px[1], px[2], px[3]],
            _ => {
                let i = usize::from(px[0]) * 3;
                let rgb = palette.get(i..i + 3)?;
                [rgb[0], rgb[1], rgb[2], 255]
            }
        };
        rgba.extend_from_slice(&pixel);
    }
    Some(Pixels {
        width,
        height,
        rgba,
    })
}
