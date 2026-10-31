//! Splitting a model that is one mesh into parts on a provider: Tripo's
//! mesh segmentation (with an optional part completion after it) and
//! Meshy's Auto Split. Both run as provider jobs: they wait for the credits
//! approval, write the credits ledger, resume after a restart and land in
//! the Inbox as the asset's next version.
//!
//! Tripo takes the model by the task that made it, or as an uploaded file
//! when it did not come from Tripo. Meshy only splits a model one of its
//! own Meshy 6 or Meshy 7 tasks made, and the split never keeps the
//! texture: the parts come back with flat vertex colours, so the texture of
//! the version it was split from bakes back onto them in Blender.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::catalog::is_smart_topology;

/// Tripo's mesh segmentation, credits per task (its published example).
pub const TRIPO_SEGMENT_CREDITS: f64 = 40.0;
/// Tripo's part completion, credits per task (its published example).
pub const TRIPO_COMPLETE_CREDITS: f64 = 100.0;
/// Meshy's Auto Split, refunded when the task fails.
pub const MESHY_SPLIT_CREDITS: f64 = 10.0;
/// Meshy's Retexture, refunded when the task fails.
pub const MESHY_RETEXTURE_CREDITS: f64 = 10.0;
/// Meshy reads a texture prompt of at most 800 characters.
pub const MAX_RETEXTURE_PROMPT: usize = 800;
/// Meshy reads 1 to 10 part names from a prompt of at most 600 characters.
pub const MAX_MESHY_PARTS: usize = 10;
pub const MAX_MESHY_PROMPT: usize = 600;
/// The Meshy models Auto Split takes.
pub const MESHY_SPLIT_MODELS: [&str; 5] =
    ["meshy-6-lite", "meshy-6", "meshy-7", "meshy-7.1", "latest"];

/// Which Tripo segmentation model cuts it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentModel {
    /// Cuts where the shape changes.
    #[default]
    Geometry,
    /// Beta: looks for parts that mean something, like a lid or a cage.
    Semantic,
}

impl SegmentModel {
    pub fn id(self) -> &'static str {
        match self {
            Self::Geometry => "v1.0-20250506",
            Self::Semantic => "v2.0-20260430",
        }
    }
}

/// How many parts the semantic model looks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Granularity {
    Simple,
    #[default]
    Balanced,
    Detailed,
}

impl Granularity {
    fn id(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Balanced => "balanced",
            Self::Detailed => "detailed",
        }
    }
}

/// How Tripo rebuilds the faces one part hid in another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionMode {
    /// Generates the missing geometry.
    AiCompletion,
    /// Only seals the open edges.
    QuickCap,
}

impl CompletionMode {
    fn id(self) -> &'static str {
        match self {
            Self::AiCompletion => "ai_completion",
            Self::QuickCap => "quick_cap",
        }
    }
}

/// A Tripo split's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TripoSplit {
    pub model: SegmentModel,
    /// Semantic only.
    pub granularity: Granularity,
    /// Semantic only: loose pieces always become their own parts.
    pub split_by_connectivity: bool,
    /// A second task that completes the parts, when set.
    pub complete: Option<CompletionMode>,
}

impl Default for TripoSplit {
    fn default() -> Self {
        Self {
            model: SegmentModel::Geometry,
            granularity: Granularity::Balanced,
            split_by_connectivity: true,
            complete: None,
        }
    }
}

/// How Meshy divides the model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum MeshySplitMode {
    /// Meshy chooses the cuts.
    #[default]
    Auto,
    /// Along the parts named.
    ByParts,
    /// Along the colour regions named, found in the source image.
    ByColor,
}

/// What happens to the texture once a Meshy split is accepted. Meshy drops it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAfter {
    /// Bakes it back from the version it came from, free, on its own.
    #[default]
    Bake,
    /// Offers a re-texture on Meshy, which asks before it spends credits.
    Retexture,
    /// Stays grey; a bake back and a re-texture stay on offer.
    Grey,
}

/// A Meshy split's settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeshySplit {
    pub mode: MeshySplitMode,
    /// The parts to find, by parts or by colour: 1 to 10 names.
    #[serde(default)]
    pub parts: Vec<String>,
}

impl MeshySplit {
    /// The names trimmed, without blanks or repeats.
    pub fn names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for name in self.parts.iter().map(|p| p.trim()) {
            if !name.is_empty() && !out.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                out.push(name.to_string());
            }
        }
        out
    }

    /// The prompt Meshy reads the part names from.
    pub fn prompt(&self) -> String {
        self.names().join(", ")
    }
}

/// What is wrong with a Meshy split's settings, as a sentence.
pub fn meshy_split_problem(split: &MeshySplit) -> Option<String> {
    if split.mode == MeshySplitMode::Auto {
        return None;
    }
    let names = split.names();
    if names.is_empty() {
        return Some("Name at least one part for Meshy to look for.".to_string());
    }
    if names.len() > MAX_MESHY_PARTS {
        return Some(format!("Meshy looks for at most {MAX_MESHY_PARTS} parts."));
    }
    if split.prompt().chars().count() > MAX_MESHY_PROMPT {
        return Some(format!(
            "Keep the part names under {MAX_MESHY_PROMPT} characters together; Meshy refuses longer ones."
        ));
    }
    None
}

/// The body of a Tripo `mesh/segment` task. The semantic settings only go
/// with the semantic model.
pub fn segment_request(split: &TripoSplit, input: &str) -> Value {
    let mut body = json!({ "input": input, "model": split.model.id() });
    if split.model == SegmentModel::Semantic {
        body["segmentation_granularity"] = json!(split.granularity.id());
        body["split_by_connectivity"] = json!(split.split_by_connectivity);
    }
    body
}

/// The body of a Tripo `mesh/complete` task on a finished segment task.
/// No part names: every part is completed.
pub fn complete_request(mode: CompletionMode, segment_task: &str) -> Value {
    json!({ "input": segment_task, "completion_mode": mode.id() })
}

/// The body of a Meshy Auto Split task: one GLB, parts where they were.
pub fn meshy_split_request(split: &MeshySplit, input_task: &str) -> Value {
    let mut body = json!({
        "input_task_id": input_task,
        "mode": match split.mode {
            MeshySplitMode::Auto => "auto",
            MeshySplitMode::ByParts => "by_parts",
            MeshySplitMode::ByColor => "by_color",
        },
        "target_formats": ["glb"],
        "layout": "assembled",
    });
    if split.mode != MeshySplitMode::Auto {
        body["prompt"] = json!(split.prompt());
    }
    body
}

/// Why Meshy cannot split a model its task `ai_model` made, if it cannot.
pub fn meshy_model_problem(ai_model: &str) -> Option<String> {
    if is_smart_topology(ai_model) {
        return Some(
            "Meshy cannot split Low Poly and Smart Topology models (meshy-t2 and meshy-t2.5)."
                .to_string(),
        );
    }
    if !MESHY_SPLIT_MODELS.contains(&ai_model) {
        return Some(format!(
            "Meshy splits only Meshy 6 and Meshy 7 models; this one was made with {ai_model}."
        ));
    }
    None
}

/// What a split should cost, from the providers' published numbers.
pub fn credits(tripo: Option<&TripoSplit>, meshy: Option<&MeshySplit>) -> Option<f64> {
    match (tripo, meshy) {
        (Some(t), _) => Some(
            TRIPO_SEGMENT_CREDITS
                + if t.complete.is_some() {
                    TRIPO_COMPLETE_CREDITS
                } else {
                    0.0
                },
        ),
        (None, Some(_)) => Some(MESHY_SPLIT_CREDITS),
        (None, None) => None,
    }
}

/// The settings in a few words: "Tripo segment, semantic, balanced, then
/// part completion" or "Meshy Auto Split, by parts, 5 names".
pub fn summary(tripo: Option<&TripoSplit>, meshy: Option<&MeshySplit>) -> String {
    match (tripo, meshy) {
        (Some(t), _) => {
            let mut parts = vec!["Tripo segment".to_string()];
            match t.model {
                SegmentModel::Geometry => parts.push("geometry".to_string()),
                SegmentModel::Semantic => {
                    parts.push("semantic".to_string());
                    parts.push(t.granularity.id().to_string());
                }
            }
            if let Some(mode) = t.complete {
                parts.push(match mode {
                    CompletionMode::AiCompletion => "then part completion".to_string(),
                    CompletionMode::QuickCap => "then a quick cap".to_string(),
                });
            }
            parts.join(", ")
        }
        (None, Some(m)) => {
            let names = m.names().len();
            match m.mode {
                MeshySplitMode::Auto => "Meshy Auto Split, auto".to_string(),
                mode => format!(
                    "Meshy Auto Split, {}, {names} {}",
                    if mode == MeshySplitMode::ByParts {
                        "by parts"
                    } else {
                        "by colour"
                    },
                    if names == 1 { "name" } else { "names" }
                ),
            }
        }
        (None, None) => "Split".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_geometry_segment_sends_only_the_input_and_the_model() {
        let body = segment_request(&TripoSplit::default(), "task_abc");
        assert_eq!(
            body,
            json!({ "input": "task_abc", "model": "v1.0-20250506" })
        );
        let semantic = TripoSplit {
            model: SegmentModel::Semantic,
            granularity: Granularity::Detailed,
            split_by_connectivity: false,
            complete: Some(CompletionMode::QuickCap),
        };
        let body = segment_request(&semantic, "file_tok");
        assert_eq!(body["model"], "v2.0-20260430");
        assert_eq!(body["segmentation_granularity"], "detailed");
        assert_eq!(body["split_by_connectivity"], false);
        assert_eq!(
            complete_request(CompletionMode::QuickCap, "task_seg"),
            json!({ "input": "task_seg", "completion_mode": "quick_cap" })
        );
        assert_eq!(credits(Some(&semantic), None), Some(140.0));
        assert_eq!(credits(Some(&TripoSplit::default()), None), Some(40.0));
    }

    #[test]
    fn a_meshy_split_names_its_parts_and_keeps_the_layout() {
        let auto = meshy_split_request(&MeshySplit::default(), "018a");
        assert_eq!(
            auto,
            json!({ "input_task_id": "018a", "mode": "auto", "target_formats": ["glb"], "layout": "assembled" })
        );
        let named = MeshySplit {
            mode: MeshySplitMode::ByParts,
            parts: vec![" lid".into(), "cage".into(), "Lid".into(), String::new()],
        };
        let body = meshy_split_request(&named, "018a");
        assert_eq!(body["mode"], "by_parts");
        assert_eq!(body["prompt"], "lid, cage");
        assert_eq!(
            summary(None, Some(&named)),
            "Meshy Auto Split, by parts, 2 names"
        );
        assert_eq!(credits(None, Some(&named)), Some(10.0));
    }

    #[test]
    fn meshy_refuses_blank_or_too_many_names_and_other_models() {
        let mut split = MeshySplit {
            mode: MeshySplitMode::ByColor,
            parts: Vec::new(),
        };
        assert!(meshy_split_problem(&split).is_some());
        split.parts = (0..11).map(|i| format!("part {i}")).collect();
        assert!(meshy_split_problem(&split).is_some());
        split.parts.truncate(3);
        assert_eq!(meshy_split_problem(&split), None);
        assert!(meshy_model_problem("meshy-t2.5").is_some());
        assert!(meshy_model_problem("meshy-5").is_some());
        assert_eq!(meshy_model_problem("meshy-6"), None);
        assert_eq!(meshy_model_problem("latest"), None);
    }
}
