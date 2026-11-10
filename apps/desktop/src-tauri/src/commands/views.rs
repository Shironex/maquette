//! Standard views and model facts: the shell side of `maquette_core::views`
//! and `maquette_core::facts`. Agents ask for both over MCP; the helpers
//! here find the asset version, its refs and the copy Build scene would
//! place, so the tools stay thin.

use std::path::PathBuf;

use maquette_core::facts::{self, FactsSource, ModelFacts};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::prepare::preview::previews_dir;
use maquette_core::prepare::{self, ModelPrep};
use maquette_core::refs::AssetRef;
use maquette_core::thumbs::source::{self, Source};
use maquette_core::views::{self, StandardView, ViewModel, Views, ViewsRequest};
use maquette_core::{Asset, AssetVersion, CoreError, WorkspaceConfig, assets, project};

use super::CommandResult;
use super::prepare::{PreviewRequest, preview_job};
use crate::session::Session;

/// The views service the app shares with the MCP tools.
#[derive(Default)]
pub struct ViewJobs(pub Views);

/// One version of an asset, found.
pub(crate) struct Found {
    pub root: PathBuf,
    pub config: WorkspaceConfig,
    pub project_dir: PathBuf,
    pub asset: Asset,
    pub version: AssetVersion,
    pub refs: Vec<AssetRef>,
}

fn version_of(asset: &Asset, wanted: Option<u32>) -> CommandResult<AssetVersion> {
    let found = match wanted {
        Some(n) => asset.versions.iter().find(|v| v.version == n),
        None => asset.latest(),
    };
    found.cloned().ok_or_else(|| {
        let message = match (wanted, asset.versions.is_empty()) {
            (_, true) => format!("{} has no model yet.", asset.name),
            (Some(n), false) => {
                let have: Vec<String> = asset
                    .versions
                    .iter()
                    .map(|v| format!("v{}", v.version))
                    .collect();
                format!(
                    "{} has no v{n}. Its versions: {}.",
                    asset.name,
                    have.join(", ")
                )
            }
            (None, false) => format!("{} has no model yet.", asset.name),
        };
        super::invalid(&message)
    })
}

/// The asset `asset_id` of `project_id` and one of its versions, the newest
/// when none is named.
pub(crate) fn find(
    session: &Session,
    project_id: &str,
    asset_id: &str,
    version: Option<u32>,
) -> CommandResult<Found> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let version = version_of(&asset, version)?;
    let refs = super::refs::asset_refs(&project_dir, &asset);
    Ok(Found {
        root,
        config,
        project_dir,
        asset,
        version,
        refs,
    })
}

/// The measured facts of one version of an asset. Reads only.
pub(crate) fn model_facts(
    session: &Session,
    project_id: &str,
    asset_id: &str,
    version: Option<u32>,
) -> CommandResult<ModelFacts> {
    let found = find(session, project_id, asset_id, version)?;
    let previews = previews_dir(&found.root.join(&found.config.layout.cache));
    Ok(facts::facts(&FactsSource {
        project: project_id,
        project_dir: &found.project_dir,
        asset: &found.asset,
        version: &found.version,
        previews: Some(&previews),
        refs: &found.refs,
    })?)
}

/// What a set of views is asked with.
pub(crate) struct ViewsAsk<'a> {
    pub project: &'a str,
    pub asset: &'a str,
    pub version: Option<u32>,
    /// Another version, rendered with the same cameras.
    pub compare: Option<u32>,
    /// The model files as they are, without fix-ups and real size.
    pub as_is: bool,
    pub views: Vec<StandardView>,
}

/// A set of views, ready to ask the service for.
pub(crate) struct ViewsPlan {
    pub project: String,
    pub found: Found,
    /// The version compared with, when side by side.
    pub compared: Option<AssetVersion>,
    /// Whether the fix-ups and the real size were applied to the models.
    pub prepared: bool,
    pub request: ViewsRequest,
}

/// Where the set of `ask` goes and the scratch folder its run uses.
pub(crate) fn folders(
    found: &Found,
    ask: &ViewsAsk<'_>,
    compare: Option<u32>,
) -> (PathBuf, PathBuf) {
    let version = found.version.version;
    let dir = views::set_dir(
        &found.project_dir,
        &found.asset.id,
        version,
        compare,
        ask.as_is,
    );
    let name = dir.file_name().map(PathBuf::from).unwrap_or_default();
    let work = found
        .root
        .join(&found.config.layout.cache)
        .join(views::VIEWS_DIR)
        .join(ask.project)
        .join(&found.asset.id)
        .join(name);
    (dir, work)
}

/// The file Blender reads for one version: the copy with the asset's
/// fix-ups and real size when it has any (the one Build scene would place,
/// made by the fix-up preview and cached with it), else the model itself.
fn readable(
    session: &Session,
    tools: &Toolbox,
    ask: &ViewsAsk<'_>,
    found: &Found,
    entry: &ModelPrep,
    version: &AssetVersion,
) -> CommandResult<PathBuf> {
    let file = found.project_dir.join(&version.file);
    if !file.is_file() {
        return Err(CoreError::NotFound(format!("the model file {}", file.display())).into());
    }
    let model = match source::pick(&file) {
        Source::Original => file,
        Source::Uncompressed(raw) => raw,
        Source::Snapshot(why) => return Err(super::invalid(&why)),
    };
    if ask.as_is || (entry.size.is_none() && entry.fixups.is_empty()) {
        return Ok(model);
    }
    let request = PreviewRequest {
        project: Some(ask.project.to_string()),
        asset: Some(found.asset.id.clone()),
        inbox: None,
        plan: entry.clone(),
    };
    let report = preview_job(session, &request)?.on(model).run(tools)?;
    Ok(PathBuf::from(report.output))
}

/// Finds the versions of `ask` and makes their prepared copies. Blocks
/// while a fix-up preview that has not run yet runs.
pub(crate) fn plan(
    session: &Session,
    tools: &Toolbox,
    ask: ViewsAsk<'_>,
    size: u32,
) -> CommandResult<ViewsPlan> {
    let found = find(session, ask.project, ask.asset, ask.version)?;
    let compared = ask
        .compare
        .map(|n| version_of(&found.asset, Some(n)))
        .transpose()?;
    if compared
        .as_ref()
        .is_some_and(|c| c.version == found.version.version)
    {
        return Err(super::invalid(
            "compare takes another version than the one rendered.",
        ));
    }
    let entry = prepare::load(&found.project_dir)?
        .models
        .get(&found.asset.id)
        .cloned()
        .unwrap_or_default();
    let prepared = !ask.as_is && (entry.size.is_some() || !entry.fixups.is_empty());
    let mut models = Vec::new();
    for version in std::iter::once(&found.version).chain(compared.as_ref()) {
        models.push(ViewModel {
            id: format!("v{}", version.version),
            version: version.version,
            file: readable(session, tools, &ask, &found, &entry, version)?,
        });
    }
    let (dir, work) = folders(&found, &ask, compared.as_ref().map(|c| c.version));
    let refs = found
        .refs
        .iter()
        .map(|r| found.project_dir.join(&r.file))
        .filter(|f| f.is_file())
        .collect();
    let project = ask.project.to_string();
    let request = ViewsRequest {
        dir,
        work,
        models,
        views: ask.views,
        refs,
        size,
    };
    Ok(ViewsPlan {
        project,
        found,
        compared,
        prepared,
        request,
    })
}

/// The folder a set of `ask` goes to, without preparing anything: for a
/// cancel.
pub(crate) fn set_folder(session: &Session, ask: &ViewsAsk<'_>) -> CommandResult<PathBuf> {
    let found = find(session, ask.project, ask.asset, ask.version)?;
    Ok(folders(&found, ask, ask.compare).0)
}
