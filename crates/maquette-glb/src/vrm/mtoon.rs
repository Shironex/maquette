//! MToon, soft: the VRM toon shader on every material, shaded from the
//! material's own base color and texture, a wide soft step between light
//! and shade, no outline, no rim, no matcap. The PBR values stay as they
//! are, for viewers that do not read MToon.

use serde_json::{Value, json};

use super::container::{set_extension, use_extension};

/// The MToon extension.
pub const VRMC_MATERIALS_MTOON: &str = "VRMC_materials_mtoon";

/// The shade is the base color times this.
const SHADE: f64 = 0.75;
/// 0 is a smooth Lambert falloff, 1 a hard toon step; soft sits between.
const TOONY: f64 = 0.5;
/// How far the ambient light evens out over the model.
const GI_EQUALIZATION: f64 = 0.9;

/// The soft MToon for one material.
fn soft(material: &Value) -> Value {
    let pbr = material.get("pbrMetallicRoughness");
    let base: Vec<f64> = pbr
        .and_then(|p| p.get("baseColorFactor"))
        .and_then(Value::as_array)
        .map(|c| c.iter().take(3).filter_map(Value::as_f64).collect())
        .filter(|c: &Vec<f64>| c.len() == 3)
        .unwrap_or_else(|| vec![1.0; 3]);
    let mut out = json!({
        "specVersion": "1.0",
        "shadeColorFactor": base.iter().map(|c| c * SHADE).collect::<Vec<_>>(),
        "shadingToonyFactor": TOONY,
        "shadingShiftFactor": 0.0,
        "giEqualizationFactor": GI_EQUALIZATION,
        "outlineWidthMode": "none",
    });
    if let Some(texture) = pbr.and_then(|p| p.get("baseColorTexture")) {
        let mut shade = json!({ "index": texture.get("index").cloned().unwrap_or(json!(0)) });
        if let Some(tex_coord) = texture.get("texCoord") {
            shade["texCoord"] = tex_coord.clone();
        }
        out["shadeMultiplyTexture"] = shade;
    }
    out
}

/// Puts soft MToon on every material. Returns how many it set.
pub(super) fn apply_soft(json: &mut Value) -> usize {
    let Some(materials) = json.get_mut("materials").and_then(Value::as_array_mut) else {
        return 0;
    };
    for material in materials.iter_mut() {
        let mtoon = soft(material);
        set_extension(material, VRMC_MATERIALS_MTOON, mtoon);
    }
    let count = materials.len();
    if count > 0 {
        use_extension(json, VRMC_MATERIALS_MTOON);
    }
    count
}
