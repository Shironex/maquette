//! The Tripo card in Tripo Studio's own words. The preset keeps the API's
//! names and values; a By hand send is made on the website, whose Image to
//! 3D panel names them differently (a tab for HD Model or Smart Mesh, "Ultra
//! Mesh Quality", "Texture Quality" in K, "Polycount", "Topology"), so the
//! card maps them here and lists the website's settings the API has no
//! field for at the values that suit Maquette.

use super::{CardRow, animation_label, on_off, row, row_note};
use crate::providers::catalog::{is_tripo_p_series, tripo_label};
use crate::providers::presets::{
    PresetVariant, TripoGeometryQuality, TripoPreset, TripoRigSpec, TripoTextureQuality,
};

/// What Studio's Polycount field gets when the preset leaves it to Tripo
/// (adaptive): Studio has no adaptive, and this is its own default.
pub const STUDIO_DEFAULT_POLYCOUNT: u32 = 20_000;

/// The range Studio's Polycount field takes.
const STUDIO_POLYCOUNT: (u32, u32) = (500, 2_000_000);

fn polycount(face_limit: u32) -> CardRow {
    if face_limit == 0 {
        return row_note(
            "Polycount",
            STUDIO_DEFAULT_POLYCOUNT.to_string(),
            "Studio has no adaptive",
        );
    }
    let (low, high) = STUDIO_POLYCOUNT;
    let typed = face_limit.clamp(low, high);
    if typed == face_limit {
        row("Polycount", typed.to_string())
    } else {
        row_note(
            "Polycount",
            typed.to_string(),
            &format!("the preset says {face_limit}, outside what Studio takes"),
        )
    }
}

fn texture_quality(quality: TripoTextureQuality) -> &'static str {
    match quality {
        TripoTextureQuality::Standard => "2K",
        TripoTextureQuality::Detailed => "4K",
        TripoTextureQuality::Extreme => "8K",
    }
}

/// The Tripo card, in Studio's labels and values, in the panel's order.
pub fn tripo_rows(preset: &TripoPreset, variant: PresetVariant, export_name: &str) -> Vec<CardRow> {
    let p_series = is_tripo_p_series(&preset.model);
    let legacy = preset.model.starts_with("v2");
    let h_series = !p_series && !legacy;
    let mut rows = Vec::new();
    if h_series {
        rows.push(row(
            "Tab",
            if preset.smart_low_poly {
                "Smart Mesh"
            } else {
                "HD Model"
            },
        ));
    }
    rows.push(row_note(
        "AI Model",
        tripo_label(&preset.model),
        &format!("model id {}", preset.model),
    ));
    if h_series {
        rows.push(row(
            "Ultra Mesh Quality",
            on_off(preset.geometry_quality == TripoGeometryQuality::Detailed),
        ));
    }
    // Studio settings the API has no field for, at what suits Maquette.
    rows.push(row_note("AI Complete", "On", "recommended"));
    rows.push(row("Texture", on_off(preset.texture)));
    if preset.texture && !legacy {
        rows.push(row(
            "Texture Quality",
            texture_quality(preset.texture_quality),
        ));
    }
    if preset.texture {
        rows.push(row_note("Remove Lighting", "On", "recommended"));
    }
    rows.push(row("PBR", on_off(preset.texture && preset.pbr)));
    rows.push(row_note("Topology", "Triangle", "quads export FBX only"));
    rows.push(polycount(preset.face_limit));
    rows.push(row_note("Generate in Parts", "Off", "recommended"));
    if variant == PresetVariant::Character && preset.rig {
        rig_rows(&mut rows, preset);
    }
    rows.push(row_note("Export", export_name, "GLB, with this file name"));
    rows
}

fn rig_rows(rows: &mut Vec<CardRow>, preset: &TripoPreset) {
    let skeleton = match preset.rig_spec {
        TripoRigSpec::Mixamo => "Mixamo skeleton",
        TripoRigSpec::Tripo => "Tripo skeleton",
    };
    rows.push(row_note(
        "Rig",
        format!("{skeleton}, export GLB (or FBX) with Export Skeleton on, Texture Resolution 2K"),
        &format!("rig model {}", preset.rig_model),
    ));
    if !preset.animations.is_empty() {
        let names: Vec<String> = preset
            .animations
            .iter()
            .map(|a| animation_label(a))
            .collect();
        rows.push(row_note(
            "Animations",
            names.join(", "),
            "retarget them onto the rig",
        ));
    }
}
