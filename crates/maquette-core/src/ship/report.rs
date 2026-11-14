//! The size report: each model as it came in against what ships, in bytes
//! and triangles, with the same table as CSV and Markdown to paste.

use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::plan::{size, triangles};
use super::{TextureFormat, load_or_default};
use crate::pipeline::PipelineDef;
use crate::project::ProjectManifest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SizeRow {
    pub asset: String,
    pub name: String,
    pub version: u32,
    /// The model the asset's newest version came in as.
    pub source_file: String,
    pub source_bytes: Option<f64>,
    pub source_triangles: Option<u32>,
    /// The compressed file Ship copies; none before Compress made it.
    pub shipped_file: String,
    pub shipped_bytes: Option<f64>,
    pub shipped_triangles: Option<u32>,
    /// The export Compress read, in the raw folder, when it is there.
    pub export_bytes: Option<f64>,
    /// Why the shipped file is larger than the source, when it is: the
    /// pipeline adds data the source lacks (Bake light's lightmaps and
    /// their UV set), which Compress then shrinks.
    pub grew: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SizeTotals {
    /// Only rows with both sides measured count, so the two compare.
    pub models: u32,
    pub source_bytes: f64,
    pub shipped_bytes: f64,
    pub source_triangles: f64,
    pub shipped_triangles: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SizeReport {
    pub project: String,
    pub textures: TextureFormat,
    pub rows: Vec<SizeRow>,
    pub totals: SizeTotals,
    /// The rows as CSV, bytes and triangles as plain numbers.
    pub csv: String,
    /// The rows as a Markdown table, sizes in KB.
    pub markdown: String,
}

fn kb(bytes: Option<f64>) -> String {
    bytes.map_or_else(|| "-".to_string(), |b| format!("{:.0} KB", b / 1024.0))
}

fn num<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(String::new, |v| v.to_string())
}

fn tris(v: Option<u32>) -> String {
    v.map_or_else(|| "-".to_string(), |v| v.to_string())
}

/// `shipped` as a share of `source`: "-42%".
fn change(source: f64, shipped: f64) -> String {
    if source <= 0.0 {
        return "-".to_string();
    }
    format!("{:+.0}%", (shipped / source - 1.0) * 100.0)
}

/// What a model file carries that the size report explains growth with.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Contents {
    images: usize,
    /// Materials with the `lightmap` extras Export GLB writes for a bake.
    lightmaps: usize,
}

fn contents(path: &Path) -> Contents {
    let json = std::fs::read(path)
        .ok()
        .and_then(|bytes| maquette_glb::motion::json_of(&bytes));
    let Some(json) = json else {
        return Contents::default();
    };
    let count = |key: &str| json[key].as_array().map_or(0, Vec::len);
    let lightmaps = json["materials"].as_array().map_or(0, |materials| {
        materials
            .iter()
            .filter(|m| m["extras"]["lightmap"].is_number())
            .count()
    });
    Contents {
        images: count("images"),
        lightmaps,
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Why `shipped` is larger than `source`, or `None` when it is not.
fn growth(
    (source, source_has): (f64, Contents),
    export: Option<f64>,
    (shipped, shipped_has): (f64, Contents),
) -> Option<String> {
    if shipped <= source {
        return None;
    }
    if let Some(export) = export.filter(|e| shipped > *e) {
        return Some(format!(
            "Compress made it larger than its {} export. Run the pipeline again to ship the export as it is.",
            kb(Some(export))
        ));
    }
    let lightmaps = shipped_has.lightmaps.saturating_sub(source_has.lightmaps);
    let images = shipped_has.images.saturating_sub(source_has.images);
    let added = if lightmaps > 0 {
        format!(
            "Bake light added {} and their UV set",
            plural(lightmaps, "lightmap", "lightmaps")
        )
    } else if images > 0 {
        format!(
            "the pipeline added {}",
            plural(images, "texture", "textures")
        )
    } else {
        "the pipeline added data the source does not have".to_string()
    };
    let kept = export.map_or_else(String::new, |export| {
        format!(
            " Compress kept {:.0}% of the {} export.",
            shipped / export * 100.0,
            kb(Some(export))
        )
    });
    Some(format!("Larger than the source: {added}.{kept}"))
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn to_csv(rows: &[SizeRow]) -> String {
    let mut out = String::from(
        "asset,name,version,source_file,source_bytes,source_triangles,shipped_file,shipped_bytes,shipped_triangles,export_bytes,grew\n",
    );
    for r in rows {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{}",
            csv_field(&r.asset),
            csv_field(&r.name),
            r.version,
            csv_field(&r.source_file),
            num(r.source_bytes),
            num(r.source_triangles),
            csv_field(&r.shipped_file),
            num(r.shipped_bytes),
            num(r.shipped_triangles),
            num(r.export_bytes),
            csv_field(r.grew.as_deref().unwrap_or_default()),
        );
    }
    out
}

/// The share column: "-42%", or "+157% larger" when the file grew.
fn size_change(source: f64, shipped: f64) -> String {
    let delta = change(source, shipped);
    if shipped > source && source > 0.0 {
        format!("{delta} larger")
    } else {
        delta
    }
}

fn to_markdown(rows: &[SizeRow], totals: &SizeTotals) -> String {
    let mut out = String::from(
        "| Model | Source | Shipped | Size | Source triangles | Shipped triangles |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for r in rows {
        let delta = match (r.source_bytes, r.shipped_bytes) {
            (Some(a), Some(b)) => size_change(a, b),
            _ => "-".to_string(),
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {delta} | {} | {} |",
            r.name.replace('|', "/"),
            kb(r.source_bytes),
            kb(r.shipped_bytes),
            tris(r.source_triangles),
            tris(r.shipped_triangles),
        );
    }
    if totals.models > 0 {
        let _ = writeln!(
            out,
            "| **All {}** | {} | {} | {} | {:.0} | {:.0} |",
            totals.models,
            kb(Some(totals.source_bytes)),
            kb(Some(totals.shipped_bytes)),
            size_change(totals.source_bytes, totals.shipped_bytes),
            totals.source_triangles,
            totals.shipped_triangles,
        );
    }
    for r in rows {
        if let Some(why) = &r.grew {
            let _ = write!(out, "\n**{}**: {why}\n", r.name.replace('|', "/"));
        }
    }
    out
}

/// The report for every asset with a model, in Board order.
pub fn size_report(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
) -> SizeReport {
    let ship = load_or_default(project_dir);
    let (raw, dist) = crate::inspect::folders(project_dir, def);
    let index = crate::site::ship_index(manifest, &ship);
    let mut rows = Vec::new();
    for asset in &manifest.assets {
        let (Some(latest), Some(entry)) = (asset.latest(), index.models.get(&asset.id)) else {
            continue;
        };
        let source = project_dir.join(&latest.file);
        let shipped = dist.join(&entry.file);
        // Export GLB writes the model under its source stem; only the
        // shipped file is renamed.
        let export = Path::new(&latest.file)
            .file_stem()
            .map(|stem| raw.join(format!("{}.glb", stem.to_string_lossy().to_lowercase())));
        let export_bytes = export.as_deref().and_then(size);
        let (source_bytes, shipped_bytes) = (size(&source), size(&shipped));
        let grew = match (source_bytes, shipped_bytes) {
            (Some(a), Some(b)) => growth(
                (a, contents(&source)),
                export_bytes,
                (b, contents(&shipped)),
            ),
            _ => None,
        };
        rows.push(SizeRow {
            asset: asset.id.clone(),
            name: asset.name.clone(),
            version: latest.version,
            source_file: latest.file.clone(),
            source_bytes,
            source_triangles: triangles(&source),
            shipped_file: entry.file.clone(),
            shipped_bytes,
            shipped_triangles: shipped_bytes.and_then(|_| triangles(&shipped)),
            export_bytes,
            grew,
        });
    }
    let mut totals = SizeTotals::default();
    for r in &rows {
        if let (Some(a), Some(b), Some(ta), Some(tb)) = (
            r.source_bytes,
            r.shipped_bytes,
            r.source_triangles,
            r.shipped_triangles,
        ) {
            totals.models += 1;
            totals.source_bytes += a;
            totals.shipped_bytes += b;
            totals.source_triangles += f64::from(ta);
            totals.shipped_triangles += f64::from(tb);
        }
    }
    SizeReport {
        project: manifest.id.clone(),
        textures: ship.textures,
        csv: to_csv(&rows),
        markdown: to_markdown(&rows, &totals),
        rows,
        totals,
    }
}
