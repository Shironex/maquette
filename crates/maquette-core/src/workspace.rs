//! The workspace: one root folder holding `maquette.json`, projects, the
//! inbox and the cache.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::fs::{clear_dir_contents, collapse_home, dir_size, expand_home};
use crate::json::{from_json_with_defaults, write_json};
use crate::providers::{ProviderSettings, meshy_problem, tripo_problem};

mod lab;
pub mod lock;
pub mod sync;

pub use lab::{LAB_MAX_CONCURRENCY, LabSettings};

pub const CONFIG_FILE: &str = "maquette.json";
pub const CONFIG_VERSION: u32 = 1;

/// Files the OS drops into folders. They do not make a folder "not empty".
const IGNORED_ENTRIES: [&str; 3] = [".DS_Store", "Thumbs.db", "desktop.ini"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    pub version: u32,
    pub layout: FolderLayout,
    pub providers: ProviderSettings,
    pub budgets: Budgets,
    pub compression: Compression,
    pub agents: AgentSettings,
    pub lab: LabSettings,
    pub current_project: Option<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            layout: FolderLayout::default(),
            providers: ProviderSettings::default(),
            budgets: Budgets::default(),
            compression: Compression::default(),
            agents: AgentSettings::default(),
            lab: LabSettings::default(),
            current_project: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderLayout {
    pub projects: String,
    pub inbox: String,
    pub cache: String,
}

impl Default for FolderLayout {
    fn default() -> Self {
        Self {
            projects: "projects".to_string(),
            inbox: "inbox".to_string(),
            cache: "cache".to_string(),
        }
    }
}

impl FolderLayout {
    fn names(&self) -> [&str; 3] {
        [&self.projects, &self.inbox, &self.cache]
    }
}

/// The mesh a provider makes: triangles or quads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Topology {
    #[default]
    Triangle,
    Quad,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum TextureSize {
    #[serde(rename = "1K")]
    K1,
    #[default]
    #[serde(rename = "2K")]
    K2,
    #[serde(rename = "4K")]
    K4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Budgets {
    pub scene_mb: f64,
    pub tris_per_model: u32,
    /// What the first camera stop's models may weigh, in MB.
    pub first_view_mb: f64,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            scene_mb: 3.0,
            tris_per_model: 60_000,
            first_view_mb: 3.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Compression {
    /// The largest texture a model may carry, checked in the Inbox and
    /// used by the compress step.
    pub texture_size: TextureSize,
    pub texture_codec: TextureCodec,
    pub meshopt: MeshoptLevel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TextureCodec {
    #[default]
    Etc1s,
    Uastc,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum MeshoptLevel {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettings {
    pub mcp_enabled: bool,
    /// u32 rather than u16 so an out of range value reaches `validate` and
    /// gets a readable message instead of a JSON error.
    pub mcp_port: u32,
    pub rules: AgentRules,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            mcp_enabled: false,
            mcp_port: 4791,
            rules: AgentRules::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentRules {
    pub blender_headless_only: bool,
    pub never_capture_screen: bool,
    pub ask_before_shipping: bool,
}

impl Default for AgentRules {
    fn default() -> Self {
        Self {
            blender_headless_only: true,
            never_capture_screen: true,
            ask_before_shipping: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RootState {
    Missing,
    Empty,
    Workspace,
    NotEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RootInspection {
    /// Expanded absolute path.
    pub path: String,
    /// The path with the home folder shown as `~`.
    pub display: String,
    pub state: RootState,
    /// Entries in the folder, not counting OS clutter such as `.DS_Store`
    /// or a sync tool's own files.
    pub entries: u32,
}

fn absolute(root: &Path, home: Option<&Path>) -> PathBuf {
    let expanded = expand_home(&root.to_string_lossy(), home);
    std::path::absolute(&expanded).unwrap_or(expanded)
}

pub fn inspect_root(root: &Path, home: Option<&Path>) -> CoreResult<RootInspection> {
    let path = absolute(root, home);
    let display = collapse_home(&path, home);
    let inspection = |state, entries| RootInspection {
        path: path.display().to_string(),
        display: display.clone(),
        state,
        entries,
    };

    let meta = match fs::metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(inspection(RootState::Missing, 0));
        }
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    if !meta.is_dir() {
        return Err(CoreError::Invalid(
            "This path is a file. Pick a folder.".to_string(),
        ));
    }

    let mut entries = 0u32;
    for entry in fs::read_dir(&path).map_err(|e| CoreError::io(&path, e))? {
        let entry = entry.map_err(|e| CoreError::io(&path, e))?;
        let name = entry.file_name();
        let synced = name.to_str().is_some_and(sync::is_sync_name);
        if !synced && !IGNORED_ENTRIES.iter().any(|ignored| name == *ignored) {
            entries = entries.saturating_add(1);
        }
    }
    let state = if path.join(CONFIG_FILE).is_file() {
        RootState::Workspace
    } else if entries == 0 {
        RootState::Empty
    } else {
        RootState::NotEmpty
    };
    Ok(inspection(state, entries))
}

/// Creates a workspace in a missing or empty folder, or opens the one that is
/// already there.
pub fn create(root: &Path, config: &WorkspaceConfig) -> CoreResult<WorkspaceConfig> {
    match inspect_root(root, None)?.state {
        RootState::Workspace => open(root),
        RootState::NotEmpty => Err(CoreError::NotEmpty(root.to_path_buf())),
        RootState::Missing | RootState::Empty => {
            validate(config)?;
            let mut config = config.clone();
            config.version = CONFIG_VERSION;
            fs::create_dir_all(root).map_err(|e| CoreError::io(root, e))?;
            ensure_layout(root, &config.layout)?;
            save(root, &config)?;
            Ok(config)
        }
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    version: u32,
}

/// Reads the config, fills in defaults for anything missing and recreates
/// missing layout folders.
pub fn open(root: &Path) -> CoreResult<WorkspaceConfig> {
    let path = root.join(CONFIG_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotAWorkspace(root.to_path_buf()));
        }
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let probe: VersionProbe = serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if probe.version > CONFIG_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: probe.version,
            supported: CONFIG_VERSION,
        });
    }
    let text = fold_legacy_tripo(&text).map_err(|e| CoreError::json(&path, e))?;
    let mut config: WorkspaceConfig =
        from_json_with_defaults(&text).map_err(|e| CoreError::json(&path, e))?;
    config.version = CONFIG_VERSION;
    validate(&config)?;
    ensure_layout(root, &config.layout)?;
    Ok(config)
}

/// Older configs kept one `tripo` block. Its values move to where they
/// live now: the texture size to `compression`, the rest to the Tripo
/// presets. A config with a `providers` block is left alone.
fn fold_legacy_tripo(text: &str) -> Result<String, serde_json::Error> {
    use serde_json::{Map, Value, json};

    let mut root: Value = serde_json::from_str(text)?;
    let Some(object) = root.as_object_mut() else {
        return Ok(text.to_string());
    };
    let Some(Value::Object(tripo)) = object.remove("tripo") else {
        return Ok(text.to_string());
    };
    if let Some(texture) = tripo.get("texture") {
        let compression = object
            .entry("compression")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(compression) = compression.as_object_mut() {
            compression
                .entry("textureSize")
                .or_insert_with(|| texture.clone());
        }
    }
    if !object.contains_key("providers") {
        let model = match tripo.get("model").and_then(Value::as_str) {
            Some("H3.0") => "v3.0-20250812",
            Some("v2.5") => "v2.5-20250123",
            _ => crate::providers::catalog::TRIPO_DEFAULT_MODEL,
        };
        let pbr = tripo.get("pbr").cloned().unwrap_or(Value::Bool(true));
        let character = tripo.get("character").and_then(Value::as_object);
        let pick = |key: &str| character.and_then(|c| c.get(key)).cloned();
        let mut tripo_character = json!({ "model": model, "pbr": pbr });
        if let Some(faces) = pick("quads") {
            tripo_character["faceLimit"] = faces;
        }
        if let Some(multi_view) = pick("multiView") {
            tripo_character["multiView"] = multi_view;
        }
        if let Some(rig) = pick("autoRig") {
            tripo_character["rig"] = rig;
        }
        if let Some(spec) = pick("rigPreset") {
            tripo_character["rigSpec"] = spec;
        }
        object.insert(
            "providers".to_string(),
            json!({
                "tripo": {
                    "prop": { "model": model, "pbr": pbr },
                    "character": tripo_character,
                }
            }),
        );
    }
    serde_json::to_string(&root)
}

fn ensure_layout(root: &Path, layout: &FolderLayout) -> CoreResult<()> {
    for name in layout.names() {
        let dir = root.join(name);
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    }
    Ok(())
}

/// Validates, then writes pretty JSON atomically.
pub fn save(root: &Path, config: &WorkspaceConfig) -> CoreResult<()> {
    validate(config)?;
    let path = root.join(CONFIG_FILE);
    write_json(&path, config)
}

/// True when `name` is one plain folder name, safe to join onto a root.
pub(crate) fn is_single_segment(name: &str) -> bool {
    const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    !name.trim().is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name.contains("..")
        && !name.contains(FORBIDDEN)
        && !name.chars().any(char::is_control)
}

pub fn validate(config: &WorkspaceConfig) -> CoreResult<()> {
    let invalid = |msg: &str| Err(CoreError::Invalid(msg.to_string()));
    if !(1024..=65535).contains(&config.agents.mcp_port) {
        return invalid("The MCP port must be between 1024 and 65535.");
    }
    config.lab.validate()?;
    let scene_mb = config.budgets.scene_mb;
    if !scene_mb.is_finite() || scene_mb <= 0.0 || scene_mb > 1000.0 {
        return invalid("The scene budget must be more than 0 MB and at most 1000 MB.");
    }
    let first_view_mb = config.budgets.first_view_mb;
    if !first_view_mb.is_finite() || first_view_mb <= 0.0 || first_view_mb > 1000.0 {
        return invalid("The first view budget must be more than 0 MB and at most 1000 MB.");
    }
    if config.budgets.tris_per_model == 0 {
        return invalid("The triangle budget per model must be more than 0.");
    }
    let providers = &config.providers;
    for preset in [&providers.tripo.prop, &providers.tripo.character] {
        if let Some(problem) = tripo_problem(preset) {
            return Err(CoreError::Invalid(problem));
        }
    }
    for preset in [&providers.meshy.prop, &providers.meshy.character] {
        if let Some(problem) = meshy_problem(preset) {
            return Err(CoreError::Invalid(problem));
        }
    }
    let names = config.layout.names();
    if names.iter().any(|name| !is_single_segment(name)) {
        return invalid("Folder names must be plain names, without slashes or dots.");
    }
    let lowered: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    if lowered[0] == lowered[1] || lowered[0] == lowered[2] || lowered[1] == lowered[2] {
        return invalid("The projects, inbox and cache folders need three different names.");
    }
    Ok(())
}

pub fn cache_size(root: &Path, config: &WorkspaceConfig) -> u64 {
    dir_size(&root.join(&config.layout.cache))
}

/// Empties the cache folder and returns the bytes freed.
pub fn clear_cache(root: &Path, config: &WorkspaceConfig) -> CoreResult<u64> {
    if !is_single_segment(&config.layout.cache) {
        return Err(CoreError::Invalid(
            "The cache folder name is not a plain name.".to_string(),
        ));
    }
    let dir = root.join(&config.layout.cache);
    clear_dir_contents(&dir).map_err(|e| CoreError::io(&dir, e))
}

#[cfg(test)]
mod tests;
