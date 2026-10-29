//! Loading buffers and images from data URIs and files, and listing a
//! model's textures.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use gltf::image::Source as ImageSource;

use super::{PbrMaps, TextureInfo};
use crate::images;

pub fn load_uri(uri: &str, base: &Path) -> Option<Vec<u8>> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (_, data) = rest.split_once(";base64,")?;
        return base64_decode(data);
    }
    if uri.contains("://") {
        return None;
    }
    let decoded = percent_decode(uri);
    let path = base.join(decoded);
    // External files must sit next to the model, never above it.
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    fs::read(path).ok()
}

pub(super) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = s.get(i + 1..i + 3)
            && let Ok(v) = u8::from_str_radix(hex, 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(super) fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(u32::from(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        }))
    };
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= value(c)? << (18 - 6 * i);
        }
        let bytes = acc.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Some(out)
}

pub(super) fn textures(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    base: &Path,
) -> (Vec<TextureInfo>, PbrMaps) {
    // Image index for a texture, including KTX2 sources behind
    // KHR_texture_basisu.
    let image_of = |texture: &gltf::Texture<'_>| -> Option<usize> {
        texture.source().map(|i| i.index()).or_else(|| {
            texture
                .extensions()?
                .get("KHR_texture_basisu")?
                .get("source")?
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
        })
    };
    let mut uses: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut pbr = PbrMaps::default();
    for material in doc.materials() {
        let m = material.pbr_metallic_roughness();
        let slots = [
            ("base color", m.base_color_texture().map(|t| t.texture())),
            (
                "metal and roughness",
                m.metallic_roughness_texture().map(|t| t.texture()),
            ),
            ("normal", material.normal_texture().map(|t| t.texture())),
            (
                "occlusion",
                material.occlusion_texture().map(|t| t.texture()),
            ),
            ("emissive", material.emissive_texture().map(|t| t.texture())),
        ];
        for (slot, texture) in slots {
            let Some(texture) = texture else { continue };
            match slot {
                "base color" => pbr.base_color = true,
                "metal and roughness" => pbr.metallic_roughness = true,
                "normal" => pbr.normal = true,
                "occlusion" => pbr.occlusion = true,
                _ => pbr.emissive = true,
            }
            if let Some(image) = image_of(&texture) {
                let list = uses.entry(image).or_default();
                if !list.iter().any(|s| s == slot) {
                    list.push(slot.to_string());
                }
            }
        }
    }
    let list = doc
        .images()
        .map(|image| {
            let (data, mime) = match image.source() {
                ImageSource::View { view, mime_type } => {
                    let data = buffers
                        .get(view.buffer().index())
                        .and_then(|b| b.as_deref())
                        .and_then(|b| b.get(view.offset()..view.offset() + view.length()))
                        .map(<[u8]>::to_vec);
                    (data, Some(mime_type.to_string()))
                }
                ImageSource::Uri { uri, mime_type } => {
                    (load_uri(uri, base), mime_type.map(str::to_string))
                }
            };
            let dims = data.as_deref().and_then(images::dimensions);
            let mime = mime.or_else(|| data.as_deref().and_then(images::sniff).map(str::to_string));
            TextureInfo {
                name: image
                    .name()
                    .map_or_else(|| format!("Image {}", image.index() + 1), str::to_string),
                mime,
                width: dims.map(|d| d.0),
                height: dims.map(|d| d.1),
                bytes: data.as_ref().map_or(0.0, |d| d.len() as f64),
                uses: uses.remove(&image.index()).unwrap_or_default(),
            }
        })
        .collect();
    (list, pbr)
}
