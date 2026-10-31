//! The Lab's image backends: Codex or OpenAI's Image API per kind of job
//! (Settings > Image generation, or an agent's override), the OpenAI key
//! in the keychain, and the month's spend against the cap.

use std::path::Path;

use maquette_core::concepts::Concepts;
use maquette_core::concepts::backend::{ApiPlan, ImageBackend, ImageJobKind, ImageSettings};
use maquette_core::concepts::jobs::ImageApiClient;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::http::Timing;
use maquette_core::providers::ledger::{self, month_to_date};
use maquette_core::providers::openai_images::{
    self, ImageQuality, OpenAiKeyTest, PRICES_CHECKED, PRICES_URL, SERVICE, estimate_usd,
};
use maquette_core::providers::secrets::{self, OPENAI_ACCOUNT, SecretStore};
use maquette_core::{CoreError, WorkspaceConfig};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};

use super::lab::Lab;
use super::providers::Studio;
use super::{CommandResult, blocking};
use crate::session::Session;

/// The Image API's address. Debug builds can point it at a local fake
/// with `MAQUETTE_OPENAI_URL`.
pub(crate) fn base_url() -> String {
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var("MAQUETTE_OPENAI_URL")
    {
        return url;
    }
    openai_images::BASE_URL.to_string()
}

/// Lets the Lab reach the Image API with the key in `secrets`.
pub(crate) fn connect(lab: &Concepts, secrets: &std::sync::Arc<dyn SecretStore>) {
    lab.set_image_api(ImageApiClient {
        secrets: std::sync::Arc::clone(secrets),
        base: base_url(),
        timing: Timing::OPENAI,
    });
}

/// How a Lab batch runs: on the backend Settings picks for its kind, or
/// the one `choice` names; `via_agent` marks an agent's spend.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Backend {
    pub choice: Option<ImageBackend>,
    pub via_agent: bool,
}

/// How a batch of `kind` runs: the backend's `choice`, else the one
/// Settings picks for the kind. `None` is Codex.
pub(crate) fn plan(
    root: &Path,
    config: &WorkspaceConfig,
    kind: ImageJobKind,
    backend: Backend,
) -> Option<ApiPlan> {
    let images = &config.lab.images;
    let via_agent = backend.via_agent;
    match backend.choice.unwrap_or_else(|| images.backends.get(kind)) {
        ImageBackend::Codex => None,
        ImageBackend::OpenaiImages => Some(ApiPlan {
            kind,
            quality: images.quality,
            monthly_cap_usd: images.monthly_cap_usd,
            ledger_root: root.to_path_buf(),
            via_agent,
            // An agent's paid round waits for a person in the app.
            hold: via_agent,
            per_image_usd: per_image_usd(root, images.quality),
        }),
    }
}

/// What one image at `quality` has cost lately, from the ledger; `None`
/// when it has too few measured images (or cannot be read).
fn per_image_usd(root: &Path, quality: ImageQuality) -> Option<f64> {
    ledger::read(root)
        .ok()
        .and_then(|l| ledger::recent_average_usd(&l.images, quality))
}

/// The saved key, as Settings shows it.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiKeyStatus {
    pub saved: bool,
    /// The last four characters.
    pub tail: Option<String>,
}

/// One image's estimated cost per quality: what it has cost lately (the
/// ledger's recent average), else the price list's guess.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageEstimates {
    pub low: f64,
    pub medium: f64,
    pub high: f64,
}

/// What Settings and the Lab show of the Image API.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageApiStatus {
    pub settings: ImageSettings,
    pub key: OpenAiKeyStatus,
    /// Dollars spent on images since the first of this month.
    pub month_spent_usd: f64,
    /// What the Image API jobs still queued or running will likely cost.
    pub pending_usd: f64,
    pub model: String,
    /// One image with no attached image, per quality.
    pub estimates: ImageEstimates,
    pub prices_checked: String,
    pub prices_url: String,
}

fn key_status(secrets: &dyn SecretStore) -> CommandResult<OpenAiKeyStatus> {
    let key = secrets.get_account(OPENAI_ACCOUNT)?;
    Ok(OpenAiKeyStatus {
        saved: key.is_some(),
        tail: key.as_deref().map(secrets::tail),
    })
}

pub(crate) fn status(
    session: &Session,
    secrets: &dyn SecretStore,
    lab: &Concepts,
) -> CommandResult<ImageApiStatus> {
    let (root, config) = session.workspace()?;
    let images = ledger::read(&root)?.images;
    let estimate = |quality| {
        ledger::recent_average_usd(&images, quality).unwrap_or_else(|| estimate_usd(quality, 0))
    };
    Ok(ImageApiStatus {
        settings: config.lab.images,
        key: key_status(secrets)?,
        month_spent_usd: month_to_date(&images, now_ms()),
        pending_usd: lab.api_pending_usd(),
        model: openai_images::MODEL.to_string(),
        estimates: ImageEstimates {
            low: estimate(ImageQuality::Low),
            medium: estimate(ImageQuality::Medium),
            high: estimate(ImageQuality::High),
        },
        prices_checked: PRICES_CHECKED.to_string(),
        prices_url: PRICES_URL.to_string(),
    })
}

/// Saves new image settings into `maquette.json`. Shared with the MCP
/// server.
pub(crate) fn save_settings(session: &Session, settings: ImageSettings) -> CommandResult<()> {
    session.update_config(|config| config.lab.images = settings)?;
    Ok(())
}

/// Reads the saved key, which can wait on a Keychain prompt.
#[tauri::command]
#[specta::specta]
pub async fn image_api_status(app: AppHandle) -> CommandResult<ImageApiStatus> {
    blocking(move || {
        status(
            &app.state::<Session>(),
            app.state::<Studio>().0.secrets().as_ref(),
            &app.state::<Lab>().0,
        )
    })
    .await?
}

/// Saves the OpenAI key in the keychain. Only its last four characters
/// come back.
#[tauri::command]
#[specta::specta]
pub async fn image_api_key_save(
    studio: State<'_, Studio>,
    key: String,
) -> CommandResult<OpenAiKeyStatus> {
    let key = secrets::clean_named_key(SERVICE, &key)?;
    let store = std::sync::Arc::clone(studio.0.secrets());
    blocking(move || {
        store.set_account(OPENAI_ACCOUNT, &key)?;
        key_status(store.as_ref())
    })
    .await?
}

#[tauri::command]
#[specta::specta]
pub async fn image_api_key_remove(studio: State<'_, Studio>) -> CommandResult<OpenAiKeyStatus> {
    let store = std::sync::Arc::clone(studio.0.secrets());
    blocking(move || {
        store.remove_account(OPENAI_ACCOUNT)?;
        key_status(store.as_ref())
    })
    .await?
}

/// Tries a key by listing the models, which costs nothing. Without `key`,
/// the saved one.
#[tauri::command]
#[specta::specta]
pub async fn image_api_key_test(
    studio: State<'_, Studio>,
    key: Option<String>,
) -> CommandResult<OpenAiKeyTest> {
    let key = match key.filter(|k| !k.trim().is_empty()) {
        Some(key) => secrets::clean_named_key(SERVICE, &key)?,
        None => {
            let store = std::sync::Arc::clone(studio.0.secrets());
            blocking(move || store.get_account(OPENAI_ACCOUNT))
                .await??
                .ok_or_else(|| CoreError::Invalid("Save an OpenAI API key first.".to_string()))?
        }
    };
    let base = base_url();
    blocking(move || openai_images::test_key(&base, &key, Timing::OPENAI)).await
}
