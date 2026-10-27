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

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn to_csv(rows: &[SizeRow]) -> String {
    let mut out = String::from(
        "asset,name,version,source_file,source_bytes,source_triangles,shipped_file,shipped_bytes,shipped_triangles\n",
    );
    for r in rows {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{}",
            csv_field(&r.asset),
            csv_field(&r.name),
            r.version,
            csv_field(&r.source_file),
            num(r.source_bytes),
            num(r.source_triangles),
            csv_field(&r.shipped_file),
            num(r.shipped_bytes),
            num(r.shipped_triangles),
        );
    }
    out
}

fn to_markdown(rows: &[SizeRow], totals: &SizeTotals) -> String {
    let mut out = String::from(
        "| Model | Source | Shipped | Size | Source triangles | Shipped triangles |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for r in rows {
        let delta = match (r.source_bytes, r.shipped_bytes) {
            (Some(a), Some(b)) => change(a, b),
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
            change(totals.source_bytes, totals.shipped_bytes),
            totals.source_triangles,
            totals.shipped_triangles,
        );
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
    let (_, dist) = crate::inspect::folders(project_dir, def);
    let index = crate::site::ship_index(manifest, &ship);
    let mut rows = Vec::new();
    for asset in &manifest.assets {
        let (Some(latest), Some(entry)) = (asset.latest(), index.models.get(&asset.id)) else {
            continue;
        };
        let source = project_dir.join(&latest.file);
        let shipped = dist.join(&entry.file);
        rows.push(SizeRow {
            asset: asset.id.clone(),
            name: asset.name.clone(),
            version: latest.version,
            source_file: latest.file.clone(),
            source_bytes: size(&source),
            source_triangles: triangles(&source),
            shipped_file: entry.file.clone(),
            shipped_bytes: size(&shipped),
            shipped_triangles: size(&shipped).and_then(|_| triangles(&shipped)),
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
