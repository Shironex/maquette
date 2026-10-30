//! The base color images of written VRM bytes, read out and swapped for
//! other ones. A swap only adds: the new image goes at the end of the BIN
//! chunk with a texture of its own, and the materials' base color and
//! MToon shade textures point at it. The original image and its texture
//! stay in the file, unused by those materials.

use serde_json::{Value, json};

use super::container::{self, invalid};
use super::mtoon::VRMC_MATERIALS_MTOON;
use maquette_base::error::CoreResult;

/// An image a material shades its base color from, as stored in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseColorImage {
    /// The index in `images`.
    pub image: usize,
    /// `image/png` or `image/jpeg`.
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// A new image for one base color image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseColorSwap {
    /// The index in `images` it stands in for.
    pub image: usize,
    /// `image/png` or `image/jpeg`.
    pub mime: String,
    pub bytes: Vec<u8>,
    /// Added to the original image's name.
    pub suffix: String,
}

fn index(value: Option<&Value>) -> Option<usize> {
    value
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
}

/// The texture a material's base color reads.
fn base_texture(material: &Value) -> Option<usize> {
    index(material.pointer("/pbrMetallicRoughness/baseColorTexture/index"))
}

/// The image a texture shows.
fn source(json: &Value, texture: usize) -> Option<usize> {
    index(json.get("textures")?.get(texture)?.get("source"))
}

/// The bytes of an image stored in a buffer view of the BIN chunk.
fn stored(json: &Value, bin: &[u8], image: usize) -> Option<(String, Vec<u8>)> {
    let entry = json.get("images")?.get(image)?;
    let mime = entry.get("mimeType")?.as_str()?;
    if mime != "image/png" && mime != "image/jpeg" {
        return None;
    }
    let view = json
        .get("bufferViews")?
        .get(index(entry.get("bufferView"))?)?;
    let buffer = index(view.get("buffer"))?;
    if buffer != 0 || json.pointer("/buffers/0/uri").is_some() {
        return None;
    }
    let offset = index(view.get("byteOffset")).unwrap_or(0);
    let length = index(view.get("byteLength"))?;
    let bytes = bin.get(offset..offset.checked_add(length)?)?;
    Some((mime.to_string(), bytes.to_vec()))
}

/// Each PNG or JPEG image a material reads its base color from, once, in
/// image order. Images outside the file or in other formats are left out.
///
/// # Errors
///
/// Refuses bytes that are not a GLB.
pub fn base_color_images(vrm: &[u8]) -> CoreResult<Vec<BaseColorImage>> {
    let (json, bin) = container::split(vrm)?;
    let mut images: Vec<usize> = json
        .get("materials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(base_texture)
        .filter_map(|t| source(&json, t))
        .collect();
    images.sort_unstable();
    images.dedup();
    Ok(images
        .into_iter()
        .filter_map(|image| {
            let (mime, bytes) = stored(&json, &bin, image)?;
            Some(BaseColorImage { image, mime, bytes })
        })
        .collect())
}

/// Points `slot` (a texture info) at `texture` when it reads `from`.
fn repoint(slot: Option<&mut Value>, from: &[usize], texture: usize) {
    if let Some(slot) = slot
        && index(slot.get("index")).is_some_and(|t| from.contains(&t))
    {
        slot["index"] = json!(texture);
    }
}

/// Swaps base color images in written VRM bytes: each swap's image is
/// added with a texture that keeps the first old texture's sampler, and
/// every material whose base color read the old image reads the new one,
/// for its MToon shade texture too.
///
/// # Errors
///
/// Refuses bytes that are not a GLB, a swap for an image the file does not
/// have, and an image that is not PNG or JPEG.
pub fn swap_base_color(vrm: &[u8], swaps: &[BaseColorSwap]) -> CoreResult<Vec<u8>> {
    let (mut json, mut bin) = container::split(vrm)?;
    for swap in swaps {
        if swap.mime != "image/png" && swap.mime != "image/jpeg" {
            return Err(invalid("A base color image is a PNG or JPEG image."));
        }
        let Some(old) = json.get("images").and_then(|i| i.get(swap.image)).cloned() else {
            return Err(invalid(format!("The file has no image {}.", swap.image)));
        };
        let from: Vec<usize> = json
            .get("textures")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, t)| index(t.get("source")) == Some(swap.image))
            .map(|(i, _)| i)
            .collect();
        let view = container::append_view(&mut json, &mut bin, &swap.bytes)?;
        let name = old.get("name").and_then(Value::as_str).unwrap_or("image");
        let image = container::push(
            &mut json,
            "images",
            json!({ "bufferView": view, "mimeType": swap.mime, "name": format!("{name}{}", swap.suffix) }),
        );
        let mut texture = json!({ "source": image });
        if let Some(sampler) = from
            .first()
            .and_then(|t| json.pointer(&format!("/textures/{t}/sampler")))
        {
            texture["sampler"] = sampler.clone();
        }
        let texture = container::push(&mut json, "textures", texture);
        let Some(materials) = json.get_mut("materials").and_then(Value::as_array_mut) else {
            continue;
        };
        for material in materials {
            if !base_texture(material).is_some_and(|t| from.contains(&t)) {
                continue;
            }
            repoint(
                material.pointer_mut("/pbrMetallicRoughness/baseColorTexture"),
                &from,
                texture,
            );
            let shade = format!("/extensions/{VRMC_MATERIALS_MTOON}/shadeMultiplyTexture");
            repoint(material.pointer_mut(&shade), &from, texture);
        }
    }
    container::join(&json, bin)
}
