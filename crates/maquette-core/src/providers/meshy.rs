//! Meshy's REST API (openapi v1): image or multi-image to 3D, rigging,
//! Auto Split, the task polls and cancel.
//!
//! Images go up as base64 data URIs, so nothing has to be public. Cancel
//! is a DELETE that only works while a task is still PENDING (Meshy
//! refunds it); a task that started answers 409.

use std::fs;
use std::path::Path;

use base64::Engine as _;
use serde_json::{Value, json};

use super::catalog::{is_smart_topology, meshy_takes_remove_lighting};
use super::http::{Api, ApiError, ApiResult, StopFlag};
use super::presets::{MeshyPreset, MeshyTexture, PoseMode};
use crate::workspace::Topology;

pub const BASE_URL: &str = "https://api.meshy.ai/openapi/v1";
pub const SERVICE: &str = "Meshy";

/// Meshy refuses images over 20 MB.
pub const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// Which endpoint a task lives under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    MultiImage,
    Rigging,
    /// Auto Split, which cuts a finished model into parts.
    Split,
}

impl Kind {
    pub fn path(self) -> &'static str {
        match self {
            Self::Image => "image-to-3d",
            Self::MultiImage => "multi-image-to-3d",
            Self::Rigging => "rigging",
            Self::Split => "print/split",
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        [Self::Image, Self::MultiImage, Self::Rigging, Self::Split]
            .into_iter()
            .find(|k| k.path() == path)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// PENDING: still cancellable.
    Pending,
    Running {
        progress: u32,
    },
    Succeeded,
    /// FAILED or CANCELED, with why.
    Over(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub status: Status,
    pub body: Value,
    pub credits: Option<f64>,
    /// How long Meshy asks to wait before the next check.
    pub retry_after: Option<std::time::Duration>,
}

/// A data URI for an image Meshy accepts: PNG, JPEG or WebP, 20 MB at most.
pub fn data_uri(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => {
            return Err(format!(
                "{} is not a PNG, JPEG or WebP image.",
                path.display()
            ));
        }
    };
    let bytes = fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(format!("{} is over 20 MB.", path.display()));
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

/// The endpoint and body for one generation. Smart Topology has no
/// multi-image endpoint, so it always sends the front image only.
pub fn generation_request(preset: &MeshyPreset, images: &[String]) -> (Kind, Value) {
    let smart = is_smart_topology(&preset.ai_model);
    let multi = preset.multi_view && images.len() >= 2 && !smart;
    let mut body = if multi {
        json!({ "image_urls": images.iter().take(4).collect::<Vec<_>>() })
    } else {
        json!({ "image_url": images.first().cloned().unwrap_or_default() })
    };
    body["ai_model"] = json!(preset.ai_model);
    if smart {
        body["model_type"] = json!("smart-topology");
        body["target_polycount"] = json!(preset.target_polycount);
    } else {
        body["should_remesh"] = json!(preset.should_remesh);
        if preset.should_remesh {
            body["topology"] = json!(match preset.topology {
                Topology::Triangle => "triangle",
                Topology::Quad => "quad",
            });
            body["target_polycount"] = json!(preset.target_polycount);
        }
    }
    body["should_texture"] = json!(preset.should_texture);
    body["enable_pbr"] = json!(preset.should_texture && preset.enable_pbr);
    if preset.should_texture {
        body["texture_resolution"] = json!(match preset.texture_resolution {
            MeshyTexture::K2 => "2k",
            MeshyTexture::K4 => "4k",
            MeshyTexture::K8 => "8k",
        });
    }
    body["pose_mode"] = json!(match preset.pose_mode {
        PoseMode::None => "",
        PoseMode::APose => "a-pose",
        PoseMode::TPose => "t-pose",
    });
    // Only meshy-6 uses it; lite and Smart Topology answer 400 to it.
    if preset.ai_model == "meshy-6" && meshy_takes_remove_lighting(&preset.ai_model) {
        body["remove_lighting"] = json!(preset.remove_lighting);
    }
    body["target_formats"] = json!(["glb"]);
    (if multi { Kind::MultiImage } else { Kind::Image }, body)
}

pub fn rigging_request(preset: &MeshyPreset, model_task: &str) -> Value {
    json!({
        "input_task_id": model_task,
        "height_meters": preset.height_meters,
    })
}

/// The GLB link of a finished task.
pub fn glb_url(kind: Kind, body: &Value) -> Option<String> {
    let found = match kind {
        Kind::Rigging => body.pointer("/result/rigged_character_glb_url"),
        Kind::Image | Kind::MultiImage | Kind::Split => body.pointer("/model_urls/glb"),
    };
    found
        .and_then(Value::as_str)
        .filter(|u| !u.is_empty())
        .map(str::to_string)
}

pub struct Meshy<'a> {
    pub api: &'a Api,
    pub stop: &'a StopFlag,
}

impl Meshy<'_> {
    fn create(&self, kind: Kind, body: &Value) -> ApiResult<String> {
        let answer = self
            .api
            .json(self.stop, || Ok(self.api.post_json(kind.path(), body)))?;
        answer
            .get("result")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| ApiError::Malformed("no task id in the answer.".to_string()))
    }

    /// Starts a generation. `prompt` guides the texture when there is one.
    pub fn generate(
        &self,
        preset: &MeshyPreset,
        images: &[String],
        prompt: Option<&str>,
    ) -> ApiResult<(Kind, String)> {
        let (kind, mut body) = generation_request(preset, images);
        if let Some(prompt) = prompt.filter(|p| preset.should_texture && !p.is_empty()) {
            body["texture_prompt"] = json!(prompt);
        }
        self.create(kind, &body).map(|id| (kind, id))
    }

    pub fn rig(&self, preset: &MeshyPreset, model_task: &str) -> ApiResult<String> {
        self.create(Kind::Rigging, &rigging_request(preset, model_task))
    }

    /// Starts an Auto Split of a finished task's model.
    pub fn split(&self, split: &super::split::MeshySplit, input_task: &str) -> ApiResult<String> {
        self.create(
            Kind::Split,
            &super::split::meshy_split_request(split, input_task),
        )
    }

    pub fn task(&self, kind: Kind, id: &str) -> ApiResult<Task> {
        let (body, retry_after) = self.api.json_wait(self.stop, || {
            Ok(self.api.get(&format!("{}/{id}", kind.path())))
        })?;
        let progress = body
            .get("progress")
            .and_then(Value::as_f64)
            .map_or(0, |p| p.clamp(0.0, 100.0) as u32);
        let reason = body
            .pointer("/task_error/message")
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty())
            .map(str::to_string);
        let status = match body.get("status").and_then(Value::as_str).unwrap_or("") {
            "PENDING" => Status::Pending,
            "IN_PROGRESS" => Status::Running { progress },
            "SUCCEEDED" => Status::Succeeded,
            "FAILED" => Status::Over(
                reason.unwrap_or_else(|| "Meshy could not make this model.".to_string()),
            ),
            "CANCELED" => Status::Over("The Meshy task was cancelled.".to_string()),
            other => Status::Over(format!("Meshy reported an unknown status {other:?}.")),
        };
        let credits = body.get("consumed_credits").and_then(Value::as_f64);
        Ok(Task {
            status,
            body,
            credits,
            retry_after,
        })
    }

    /// Cancels a PENDING task. A started one answers 409, which comes back
    /// as [`ApiError::Conflict`] with a plain message.
    pub fn cancel(&self, kind: Kind, id: &str) -> ApiResult<()> {
        let response = self.api.send(self.stop, || {
            Ok(self.api.delete(&format!("{}/{id}", kind.path())))
        })?;
        match response.status().as_u16() {
            200..=299 => Ok(()),
            409 => Err(ApiError::Conflict(
                "Meshy has started this job, so it cannot be cancelled and the credits are spent. \
                 It will still arrive in the Inbox."
                    .to_string(),
            )),
            401 => Err(ApiError::Auth),
            status => Err(ApiError::Status {
                status,
                message: String::new(),
            }),
        }
    }

    pub fn balance(&self) -> ApiResult<f64> {
        let answer = self.api.json(self.stop, || Ok(self.api.get("balance")))?;
        answer
            .get("balance")
            .and_then(Value::as_f64)
            .ok_or_else(|| ApiError::Malformed("no balance in the answer.".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_and_smart_topology_bodies() {
        let preset = MeshyPreset::prop();
        let (kind, body) = generation_request(&preset, &["data:a".to_string()]);
        assert_eq!(kind, Kind::Image);
        assert_eq!(body["ai_model"], "latest");
        assert_eq!(body["target_formats"], json!(["glb"]));
        assert_eq!(body["should_remesh"], false);
        assert!(body.get("model_type").is_none());
        assert!(body.get("remove_lighting").is_none());
        assert!(body.get("target_polycount").is_none());

        let mut smart = MeshyPreset::character();
        smart.ai_model = "meshy-t2.5".into();
        smart.target_polycount = 12_000;
        let (kind, body) = generation_request(&smart, &["a".to_string(), "b".to_string()]);
        assert_eq!(
            kind,
            Kind::Image,
            "Smart Topology has no multi-image endpoint"
        );
        assert_eq!(body["model_type"], "smart-topology");
        assert_eq!(body["target_polycount"], 12_000);
        assert!(body.get("should_remesh").is_none());
        assert_eq!(body["pose_mode"], "a-pose");

        let mut six = MeshyPreset::character();
        six.ai_model = "meshy-6".into();
        let (kind, body) = generation_request(&six, &["a".to_string(), "b".to_string()]);
        assert_eq!(kind, Kind::MultiImage);
        assert_eq!(body["image_urls"], json!(["a", "b"]));
        assert_eq!(body["remove_lighting"], true);
        assert_eq!(body["topology"], "triangle");
        assert!(!body.to_string().contains("lowpoly"));
    }

    #[test]
    fn data_uris_check_the_type_and_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let png = dir.path().join("front.png");
        fs::write(&png, b"\x89PNG").expect("write");
        assert!(
            data_uri(&png)
                .expect("uri")
                .starts_with("data:image/png;base64,")
        );
        let gif = dir.path().join("front.gif");
        fs::write(&gif, b"GIF8").expect("write");
        assert!(data_uri(&gif).is_err());
    }

    #[test]
    fn glb_links_by_kind() {
        let body =
            json!({ "model_urls": { "glb": "g" }, "result": { "rigged_character_glb_url": "r" } });
        assert_eq!(glb_url(Kind::Image, &body).as_deref(), Some("g"));
        assert_eq!(glb_url(Kind::Rigging, &body).as_deref(), Some("r"));
        assert_eq!(Kind::from_path("rigging"), Some(Kind::Rigging));
    }
}
