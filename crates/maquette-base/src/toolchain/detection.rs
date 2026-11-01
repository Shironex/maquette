//! Probing the candidates of a tool and turning what happened into a
//! report with a hint.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::lookup::{MAX_PROBES, candidates, find_npx, resolve_override};
use super::{
    CommandRunner, HostOs, ProbeEnv, ProbeError, ToolId, ToolReport, ToolSource, ToolStatus,
    version,
};

fn missing_hint(tool: ToolId, os: HostOs) -> &'static str {
    let windows = os == HostOs::Windows;
    match tool {
        ToolId::Blender if windows => {
            "Install Blender from blender.org or with winget install BlenderFoundation.Blender, or locate blender.exe."
        }
        ToolId::Blender if os == HostOs::Mac => {
            "Install Blender from blender.org or with brew install --cask blender, or locate Blender.app."
        }
        ToolId::Blender => "Install Blender from blender.org, or locate the blender binary.",
        ToolId::Codex => "Install it with npm i -g @openai/codex, then run codex login.",
        ToolId::Ffmpeg if windows => "Install it with winget install Gyan.FFmpeg.",
        ToolId::Ffmpeg if os == HostOs::Mac => "Install it with brew install ffmpeg.",
        ToolId::Ffmpeg => "Install ffmpeg with your package manager.",
        ToolId::Toktx => {
            "Install KTX-Software, then Re-check. Until then Compress keeps textures as PNG, so models download larger and use more GPU memory."
        }
        ToolId::GltfTransform => {
            "Install it with npm i -g @gltf-transform/cli, or install Node so I can use npx."
        }
        ToolId::Node if windows => {
            "Install Node 22 from nodejs.org or with winget install OpenJS.NodeJS.LTS."
        }
        ToolId::Node if os == HostOs::Mac => {
            "Install Node 22 from nodejs.org or with brew install node."
        }
        ToolId::Node => "Install Node 22 from nodejs.org or with your package manager.",
    }
}

pub(super) const BROKEN_HINT: &str = "Locate a working copy, then Re-check.";
const STALE_OVERRIDE: &str =
    "The saved location no longer works, so Maquette looked for another copy.";
pub(super) const CODEX_NOT_SIGNED_IN: &str = "not signed in, run codex login";

/// Finds `tool`, runs its version probe and reports what happened.
pub fn detect(
    tool: ToolId,
    env: &ProbeEnv,
    override_path: Option<&Path>,
    runner: &dyn CommandRunner,
) -> ToolReport {
    let mut warning = None;
    let mut resolved = None;
    if let Some(path) = override_path {
        match resolve_override(tool, path, env) {
            Some(found) => resolved = Some((found, ToolSource::Override)),
            None => warning = Some(STALE_OVERRIDE.to_string()),
        }
    }
    let mut report = match resolved {
        Some((path, source)) => probe(tool, env, path, source, runner),
        None => probe_candidates(tool, env, runner),
    };
    if report.warning.is_none() {
        report.warning = warning;
    }
    report
}

/// Probes copies in order and keeps the first that works, so a broken
/// global install does not hide a working one later on PATH. When none
/// works, the first failure is reported.
fn probe_candidates(tool: ToolId, env: &ProbeEnv, runner: &dyn CommandRunner) -> ToolReport {
    let mut first_broken = None;
    for (path, source) in candidates(tool, env).into_iter().take(MAX_PROBES) {
        let report = probe(tool, env, path, source, runner);
        if report.status == ToolStatus::Found {
            return report;
        }
        first_broken.get_or_insert(report);
    }
    if let Some(report) = first_broken {
        return report;
    }
    match (tool, find_npx(env)) {
        (ToolId::GltfTransform, Some(npx)) => {
            let mut report = ToolReport::new(tool, ToolStatus::Found);
            report.source = Some(ToolSource::Npx);
            report.path = Some(npx.display().to_string());
            report.detail = Some("via npx, @gltf-transform/cli".to_string());
            report
        }
        _ => {
            let mut report = ToolReport::new(tool, ToolStatus::Missing);
            report.detail = Some("not found on PATH".to_string());
            report.hint = Some(missing_hint(tool, env.os).to_string());
            report
        }
    }
}

fn probe(
    tool: ToolId,
    env: &ProbeEnv,
    path: PathBuf,
    source: ToolSource,
    runner: &dyn CommandRunner,
) -> ToolReport {
    let args = tool.version_args();
    let timeout = tool.timeout();
    let path_env = env.child_path(path.parent());
    let outcome = runner.run(&path, args, &path_env, timeout);

    let mut report = ToolReport::new(tool, ToolStatus::Found);
    report.source = Some(source);
    report.path = Some(path.display().to_string());

    let version_flag = args.last().copied().unwrap_or("--version");
    let broken = |mut report: ToolReport, detail: String| {
        report.status = ToolStatus::Broken;
        report.detail = Some(detail);
        report.hint = Some(BROKEN_HINT.to_string());
        report
    };
    match outcome {
        Err(ProbeError::Timeout) => broken(
            report,
            format!("did not answer {version_flag} in {} s", timeout.as_secs()),
        ),
        Err(ProbeError::Spawn(_)) => broken(report, "could not be started".to_string()),
        Ok(out) if !out.status_ok => broken(report, "exited with an error".to_string()),
        Ok(out) => {
            report.version = version::parse(tool, &out.stdout, &out.stderr);
            match tool {
                ToolId::Blender => report.detail = Some("runs headless".to_string()),
                ToolId::Codex => {
                    let signed_in = env.codex_auth_file().is_some_and(|f| f.is_file());
                    if signed_in {
                        report.detail = Some("signed in".to_string());
                    } else {
                        report.detail = Some(CODEX_NOT_SIGNED_IN.to_string());
                        report.warning = Some("Codex CLI is not signed in.".to_string());
                        report.hint =
                            Some("Run codex login in a terminal, then Re-check.".to_string());
                    }
                }
                _ => {}
            }
            report
        }
    }
}

/// Probes every tool in parallel and returns reports in [`ToolId::ALL`] order.
pub fn detect_all(
    env: &ProbeEnv,
    overrides: &BTreeMap<ToolId, PathBuf>,
    runner: &dyn CommandRunner,
) -> Vec<ToolReport> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = ToolId::ALL
            .iter()
            .map(|&tool| {
                let override_path = overrides.get(&tool).map(PathBuf::as_path);
                (
                    tool,
                    scope.spawn(move || detect(tool, env, override_path, runner)),
                )
            })
            .collect();
        handles
            .into_iter()
            .map(|(tool, handle)| {
                handle.join().unwrap_or_else(|_| {
                    let mut report = ToolReport::new(tool, ToolStatus::Broken);
                    report.detail = Some("the check failed".to_string());
                    report.hint = Some(BROKEN_HINT.to_string());
                    report
                })
            })
            .collect()
    })
}

/// True when a required tool is not usable, so setup cannot go on.
pub fn blocks_continue(reports: &[ToolReport]) -> bool {
    reports
        .iter()
        .any(|r| r.required && r.status != ToolStatus::Found)
}
