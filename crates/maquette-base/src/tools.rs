//! The programs a run uses, resolved once from the toolchain check.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::toolchain::{ProbeEnv, ToolId, ToolReport, ToolSource, ToolStatus, find_on_path};

/// The gltf-transform release npx runs when no global install exists.
pub const GLTF_TRANSFORM_NPX: &str = "@gltf-transform/cli@4.5.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StepKind {
    Blender,
    Shell,
    GltfTransform,
    Toktx,
    Ffmpeg,
}

impl StepKind {
    pub fn label(self) -> &'static str {
        match self {
            StepKind::Blender => "Blender",
            StepKind::Shell => "Shell",
            StepKind::GltfTransform => "gltf-transform",
            StepKind::Toktx => "toktx",
            StepKind::Ffmpeg => "ffmpeg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTool {
    pub program: PathBuf,
    /// Args that go before the step's own, for example npx's package spec.
    pub prefix: Vec<String>,
    /// Part of the cache key, so a tool upgrade reruns its steps.
    pub version: String,
}

/// Programs keyed by tool, plus the PATH children get.
#[derive(Debug, Clone, Default)]
pub struct Toolbox {
    pub tools: BTreeMap<ToolId, ResolvedTool>,
    pub path_env: OsString,
    pub os: Option<crate::toolchain::HostOs>,
}

impl Toolbox {
    /// Keeps every tool the check found working.
    pub fn from_reports(reports: &[ToolReport], env: &ProbeEnv) -> Self {
        let mut tools = BTreeMap::new();
        for report in reports.iter().filter(|r| r.status == ToolStatus::Found) {
            let Some(path) = report.path.as_deref() else {
                continue;
            };
            let npx = report.source == Some(ToolSource::Npx);
            let version = report.version.clone().unwrap_or_else(|| {
                if npx {
                    GLTF_TRANSFORM_NPX.to_string()
                } else {
                    "unknown".to_string()
                }
            });
            tools.insert(
                report.id,
                ResolvedTool {
                    program: PathBuf::from(path),
                    prefix: if npx {
                        vec!["--yes".to_string(), GLTF_TRANSFORM_NPX.to_string()]
                    } else {
                        Vec::new()
                    },
                    version,
                },
            );
        }
        let first = tools
            .get(&ToolId::Node)
            .and_then(|t| t.program.parent().map(Path::to_path_buf));
        Self {
            tools,
            path_env: env.child_path(first.as_deref()),
            os: Some(env.os),
        }
    }

    pub fn tool_for(kind: StepKind) -> Option<ToolId> {
        match kind {
            StepKind::Blender => Some(ToolId::Blender),
            StepKind::GltfTransform => Some(ToolId::GltfTransform),
            StepKind::Toktx => Some(ToolId::Toktx),
            StepKind::Ffmpeg => Some(ToolId::Ffmpeg),
            StepKind::Shell => None,
        }
    }

    pub fn has(&self, tool: ToolId) -> bool {
        self.tools.contains_key(&tool)
    }

    /// The program for a step. Shell steps look their command up on the
    /// child PATH (a command with a slash is taken as a path under `root`).
    pub fn resolve(
        &self,
        kind: StepKind,
        command: Option<&str>,
        root: &Path,
    ) -> CoreResult<ResolvedTool> {
        if let Some(tool) = Self::tool_for(kind) {
            return self.tools.get(&tool).cloned().ok_or_else(|| {
                CoreError::Unavailable(format!(
                    "{} is not installed or did not pass the tool check. Fix it in Settings, then run again.",
                    tool.label()
                ))
            });
        }
        let command = command
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| CoreError::Invalid("This shell step has no command.".to_string()))?;
        if command.contains('/') || command.contains('\\') {
            let path = root.join(command);
            return Ok(ResolvedTool {
                program: path,
                prefix: Vec::new(),
                version: String::new(),
            });
        }
        let mut probe = ProbeEnv::empty(self.os.unwrap_or_else(crate::toolchain::HostOs::current));
        probe.path_dirs = env::split_paths(&self.path_env).collect();
        if probe.os == crate::toolchain::HostOs::Windows {
            probe.pathext = env::var("PATHEXT")
                .map(|v| {
                    v.split(';')
                        .filter(|e| !e.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
        }
        find_on_path(&[command], &probe)
            .map(|program| ResolvedTool {
                program,
                prefix: Vec::new(),
                version: String::new(),
            })
            .ok_or_else(|| CoreError::Unavailable(format!("Could not find {command} on PATH.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::HostOs;

    fn report(id: ToolId, path: &str, source: ToolSource, version: Option<&str>) -> ToolReport {
        ToolReport {
            id,
            label: id.label().into(),
            required: id.required(),
            status: ToolStatus::Found,
            source: Some(source),
            path: Some(path.into()),
            version: version.map(str::to_string),
            detail: None,
            hint: None,
            warning: None,
        }
    }

    #[test]
    fn npx_gltf_transform_gets_the_pinned_package() {
        let reports = vec![
            report(
                ToolId::Blender,
                "/apps/Blender",
                ToolSource::KnownLocation,
                Some("4.3.2"),
            ),
            report(ToolId::GltfTransform, "/bin/npx", ToolSource::Npx, None),
        ];
        let mut missing = report(ToolId::Toktx, "/x", ToolSource::Path, None);
        missing.status = ToolStatus::Missing;
        let mut all = reports.clone();
        all.push(missing);
        let tools = Toolbox::from_reports(&all, &ProbeEnv::empty(HostOs::Mac));
        let gt = tools
            .resolve(StepKind::GltfTransform, None, Path::new("/p"))
            .expect("gltf-transform");
        assert_eq!(gt.program, PathBuf::from("/bin/npx"));
        assert_eq!(
            gt.prefix,
            vec!["--yes".to_string(), GLTF_TRANSFORM_NPX.to_string()]
        );
        assert_eq!(
            tools
                .resolve(StepKind::Blender, None, Path::new("/p"))
                .expect("blender")
                .version,
            "4.3.2"
        );
        assert!(!tools.has(ToolId::Toktx));
        assert_eq!(
            tools
                .resolve(StepKind::Toktx, None, Path::new("/p"))
                .expect_err("missing")
                .code(),
            "unavailable"
        );
    }

    #[cfg(unix)]
    #[test]
    fn shell_commands_resolve_on_the_child_path() {
        let tools = Toolbox {
            path_env: OsString::from("/bin:/usr/bin"),
            os: Some(HostOs::current()),
            ..Toolbox::default()
        };
        let sh = tools
            .resolve(StepKind::Shell, Some("sh"), Path::new("/p"))
            .expect("sh");
        assert!(sh.program.ends_with("sh"));
        let local = tools
            .resolve(StepKind::Shell, Some("./tools/run"), Path::new("/p"))
            .expect("relative");
        assert_eq!(local.program, PathBuf::from("/p/./tools/run"));
        assert!(
            tools
                .resolve(
                    StepKind::Shell,
                    Some("definitely-not-a-tool"),
                    Path::new("/p")
                )
                .is_err()
        );
        assert!(
            tools
                .resolve(StepKind::Shell, None, Path::new("/p"))
                .is_err()
        );
    }
}
