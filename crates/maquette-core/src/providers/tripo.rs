//! Tripo's REST API (v3): upload, image or multiview to model, the task
//! poll, the character chain (rig check, rig, retarget) and the split into
//! parts (mesh segment, part completion).
//!
//! Tripo has no cancel endpoint, so a sent task always finishes on their
//! side. Every answer is wrapped as `{"code": 0, "data": {...}}`.

use std::path::Path;

use reqwest::blocking::multipart::Form;
use serde_json::{Value, json};

use super::catalog::is_tripo_p_series;
use super::http::{Api, ApiError, ApiResult, StopFlag};
use super::presets::{TripoGeometryQuality, TripoPreset, TripoRigSpec, TripoTextureQuality};

/// The international platform, where keys from platform.tripo3d.ai work.
/// `tripo3d.com` is the China platform: its keys are separate, and an
/// international key gets 401 "Invalid API key" there.
pub const BASE_URL: &str = "https://openapi.tripo3d.ai/v3";
pub const SERVICE: &str = "Tripo";

/// The views Tripo's multiview endpoint takes, in the order Maquette sends
/// them: the first image is the front.
pub const VIEWS: [&str; 4] = ["front", "left", "back", "right"];

/// How a task stands.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Running {
        progress: u32,
    },
    Success,
    /// Failed, cancelled, banned or expired, with why.
    Over(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub status: Status,
    pub output: Value,
    pub credits: Option<f64>,
}

fn data(body: Value) -> ApiResult<Value> {
    let code = body.get("code").and_then(Value::as_i64).unwrap_or(0);
    if code != 0 {
        let message = body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        return Err(ApiError::Status {
            status: 200,
            message: format!("code {code} {message}").trim().to_string(),
        });
    }
    body.get("data")
        .cloned()
        .ok_or_else(|| ApiError::Malformed("no data in the answer.".to_string()))
}

fn string_at(value: &Value, key: &str) -> ApiResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ApiError::Malformed(format!("no {key} in the answer.")))
}

fn legacy(model: &str) -> bool {
    model.starts_with("v2")
}

/// The endpoint and body for one generation. Several images go to the
/// multiview endpoint when the preset asks for it, each under its view
/// (`views`, from the asset's tags; empty takes [`VIEWS`] in order). P
/// series and older models only get the parameters they accept.
pub fn generation_request(
    preset: &TripoPreset,
    inputs: &[String],
    views: &[String],
) -> (String, Value) {
    let model = preset.model.as_str();
    let p_series = is_tripo_p_series(model);
    let multi = preset.multi_view && inputs.len() >= 2;
    let mut body = json!({ "model": model });
    if multi {
        let named: Vec<&str> = if views.len() == inputs.len() {
            views.iter().map(String::as_str).collect()
        } else {
            VIEWS.to_vec()
        };
        let views: Vec<Value> = inputs
            .iter()
            .zip(named)
            .map(|(token, view)| json!({ view: token }))
            .collect();
        body["inputs"] = Value::Array(views);
    } else {
        body["input"] = json!(inputs.first().cloned().unwrap_or_default());
        body["enable_image_autofix"] = json!(preset.image_autofix);
    }
    if preset.face_limit > 0 {
        body["face_limit"] = json!(preset.face_limit);
    }
    body["texture"] = json!(preset.texture);
    body["pbr"] = json!(preset.texture && preset.pbr);
    if !legacy(model) {
        if preset.texture {
            body["texture_quality"] = json!(match preset.texture_quality {
                TripoTextureQuality::Standard => "standard",
                TripoTextureQuality::Detailed => "detailed",
                TripoTextureQuality::Extreme => "extreme",
            });
        }
        body["auto_size"] = json!(preset.auto_size);
        if !p_series {
            body["geometry_quality"] = json!(match preset.geometry_quality {
                TripoGeometryQuality::Standard => "standard",
                TripoGeometryQuality::Detailed => "detailed",
            });
            if preset.smart_low_poly {
                body["smart_low_poly"] = json!(true);
            }
        }
    }
    let path = if multi {
        "generation/multiview-to-model"
    } else {
        "generation/image-to-model"
    };
    (path.to_string(), body)
}

pub fn rig_request(preset: &TripoPreset, model_task: &str, rig_type: &str) -> Value {
    json!({
        "input": model_task,
        "model": preset.rig_model,
        "rig_type": rig_type,
        "spec": match preset.rig_spec {
            TripoRigSpec::Mixamo => "mixamo",
            TripoRigSpec::Tripo => "tripo",
        },
        "out_format": "glb",
    })
}

pub fn retarget_request(rig_task: &str, animations: &[String]) -> Value {
    let animations: Vec<&String> = animations.iter().take(5).collect();
    json!({
        "input": rig_task,
        "animations": animations,
        "out_format": "glb",
    })
}

/// The model link in a task's output: `model_url`, or the first of
/// `model_urls` (a string, a list or a map of formats).
pub fn model_url(output: &Value) -> Option<String> {
    if let Some(url) = output.get("model_url").and_then(Value::as_str) {
        return Some(url.to_string());
    }
    match output.get("model_urls")? {
        Value::String(url) => Some(url.clone()),
        Value::Array(list) => list.iter().find_map(|v| v.as_str().map(str::to_string)),
        Value::Object(map) => map
            .get("glb")
            .or_else(|| map.values().next())
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

pub struct Tripo<'a> {
    pub api: &'a Api,
    pub stop: &'a StopFlag,
}

impl Tripo<'_> {
    /// Uploads one file (an image, or a model to split) and returns its
    /// file token.
    pub fn upload(&self, path: &Path) -> ApiResult<String> {
        let body = self.api.json(self.stop, || {
            let form = Form::new().file("file", path).map_err(|e| {
                ApiError::Malformed(format!("could not read {}: {e}", path.display()))
            })?;
            Ok(self.api.post("files").multipart(form))
        })?;
        string_at(&data(body)?, "file_token")
    }

    fn create(&self, path: &str, body: &Value) -> ApiResult<String> {
        let answer = self
            .api
            .json(self.stop, || Ok(self.api.post_json(path, body)))?;
        string_at(&data(answer)?, "task_id")
    }

    pub fn generate(
        &self,
        preset: &TripoPreset,
        tokens: &[String],
        views: &[String],
    ) -> ApiResult<String> {
        let (path, body) = generation_request(preset, tokens, views);
        self.create(&path, &body)
    }

    pub fn rig_check(&self, model_task: &str) -> ApiResult<String> {
        self.create("animations/rig-check", &json!({ "input": model_task }))
    }

    pub fn rig(&self, preset: &TripoPreset, model_task: &str, rig_type: &str) -> ApiResult<String> {
        self.create("animations/rig", &rig_request(preset, model_task, rig_type))
    }

    pub fn retarget(&self, rig_task: &str, animations: &[String]) -> ApiResult<String> {
        self.create(
            "animations/retarget",
            &retarget_request(rig_task, animations),
        )
    }

    /// Starts a mesh segmentation of `input`: a task id or a file token.
    pub fn segment(&self, split: &super::split::TripoSplit, input: &str) -> ApiResult<String> {
        self.create("mesh/segment", &super::split::segment_request(split, input))
    }

    /// Completes every part of a finished segmentation.
    pub fn complete(
        &self,
        mode: super::split::CompletionMode,
        segment_task: &str,
    ) -> ApiResult<String> {
        self.create(
            "mesh/complete",
            &super::split::complete_request(mode, segment_task),
        )
    }

    pub fn task(&self, id: &str) -> ApiResult<Task> {
        let answer = self
            .api
            .json(self.stop, || Ok(self.api.get(&format!("tasks/{id}"))))?;
        let data = data(answer)?;
        let progress = data
            .get("progress")
            .and_then(Value::as_f64)
            .map_or(0, |p| p.clamp(0.0, 100.0) as u32);
        let reason = || {
            data.get("error_message")
                .and_then(Value::as_str)
                .filter(|m| !m.is_empty())
                .map(str::to_string)
        };
        let status = match data.get("status").and_then(Value::as_str).unwrap_or("") {
            "queued" | "running" => Status::Running { progress },
            "success" => Status::Success,
            "failed" => Status::Over(
                reason().unwrap_or_else(|| "Tripo could not make this model.".to_string()),
            ),
            "cancelled" => Status::Over("Tripo cancelled the task.".to_string()),
            "banned" => {
                Status::Over("Tripo refused the image under its content policy.".to_string())
            }
            "expired" => Status::Over("The Tripo task expired before it finished.".to_string()),
            other => Status::Over(format!("Tripo reported an unknown status {other:?}.")),
        };
        Ok(Task {
            status,
            output: data.get("output").cloned().unwrap_or(Value::Null),
            credits: data.get("credits_consumed").and_then(Value::as_f64),
        })
    }

    /// The free balance check: credits left, and those held by running tasks.
    pub fn balance(&self) -> ApiResult<(f64, f64)> {
        let answer = self
            .api
            .json(self.stop, || Ok(self.api.get("account/balance")))?;
        let data = data(answer)?;
        let number = |key| data.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        Ok((number("balance"), number("frozen")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_body_always_names_the_model_and_skips_what_a_model_refuses() {
        let mut preset = TripoPreset::prop();
        let (path, body) = generation_request(&preset, &["file_a".to_string()], &[]);
        assert_eq!(path, "generation/image-to-model");
        assert_eq!(body["model"], "v3.1-20260211");
        assert_eq!(body["input"], "file_a");
        assert_eq!(body["geometry_quality"], "standard");
        assert!(body.get("face_limit").is_none());
        assert!(body.get("quad").is_none());

        preset.model = "P1-20260311".into();
        preset.smart_low_poly = true;
        preset.face_limit = 5000;
        let (_, body) = generation_request(&preset, &["file_a".to_string()], &[]);
        for key in [
            "quad",
            "smart_low_poly",
            "generate_parts",
            "geometry_quality",
        ] {
            assert!(body.get(key).is_none(), "{key} sent to the P series");
        }
        assert_eq!(body["face_limit"], 5000);

        preset.model = "v2.5-20250123".into();
        let (_, body) = generation_request(&preset, &["file_a".to_string()], &[]);
        assert!(body.get("texture_quality").is_none());
        assert!(body.get("auto_size").is_none());
    }

    #[test]
    fn several_images_go_to_the_multiview_endpoint_front_first() {
        let preset = TripoPreset::character();
        let tokens = ["f".to_string(), "l".to_string(), "b".to_string()];
        let (path, body) = generation_request(&preset, &tokens, &[]);
        assert_eq!(path, "generation/multiview-to-model");
        assert_eq!(
            body["inputs"],
            json!([{ "front": "f" }, { "left": "l" }, { "back": "b" }])
        );
        assert!(body.get("enable_image_autofix").is_none());
        // One image falls back to the single image endpoint.
        let (path, _) = generation_request(&preset, &tokens[..1], &[]);
        assert_eq!(path, "generation/image-to-model");
    }

    #[test]
    fn model_links_come_from_either_field() {
        assert_eq!(
            model_url(&json!({ "model_url": "a" })).as_deref(),
            Some("a")
        );
        assert_eq!(
            model_url(&json!({ "model_urls": ["b", "c"] })).as_deref(),
            Some("b")
        );
        assert_eq!(
            model_url(&json!({ "model_urls": { "fbx": "x", "glb": "g" } })).as_deref(),
            Some("g")
        );
        assert_eq!(model_url(&json!({})), None);
        let retarget =
            retarget_request("task_r", &(0..7).map(|i| i.to_string()).collect::<Vec<_>>());
        assert_eq!(retarget["animations"].as_array().map(Vec::len), Some(5));
    }
}
