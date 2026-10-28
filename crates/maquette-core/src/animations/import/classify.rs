//! Deciding what each read clip of an import does: new, a duplicate of
//! keys the library (or an earlier file) has, or a revision of a clip of the
//! same name in the same pack. Pure: the files were read already.

use std::collections::BTreeMap;

use super::{
    ClipStatus, ImportFile, ImportPlan, ImportRequest, LeftOut, MatchedClip, PlanClip, StagedFile,
    TagCount,
};
use crate::animations::blender::{FileReport, ReadReport};
use crate::animations::{AnimationClip, names, rig};

/// The packs to use: the request's, else each zip's name and Singles.
fn packs_for(staged: &[StagedFile], request: Option<&ImportRequest>) -> BTreeMap<String, String> {
    let singles = request
        .and_then(|r| r.singles.as_deref())
        .and_then(names::clean_pack)
        .unwrap_or_else(|| crate::animations::SINGLES.to_string());
    staged
        .iter()
        .map(|f| {
            let chosen = request
                .and_then(|r| r.packs.iter().find(|p| p.file == f.name))
                .and_then(|p| names::clean_pack(&p.pack));
            let pack = chosen.unwrap_or_else(|| {
                if f.archive {
                    names::pack_name(&f.name)
                } else {
                    singles.clone()
                }
            });
            (f.name.clone(), pack)
        })
        .collect()
}

fn common_fps(clips: &[PlanClip]) -> Option<f32> {
    let first = clips.first()?.fps;
    clips
        .iter()
        .all(|c| (c.fps - first).abs() < 0.01)
        .then_some(first)
}

/// Decides what each read clip does. Pure: the files were read already.
pub(crate) fn classify(
    token: &str,
    staged: &[StagedFile],
    report: &ReadReport,
    existing: &[AnimationClip],
    request: Option<&ImportRequest>,
) -> ImportPlan {
    let packs = packs_for(staged, request);
    let by_id: BTreeMap<&str, &FileReport> =
        report.files.iter().map(|f| (f.id.as_str(), f)).collect();
    let mut seen: Vec<(String, MatchedClip)> = Vec::new();
    let mut files = Vec::new();
    for file in staged {
        let pack = packs.get(&file.name).cloned().unwrap_or_default();
        let mut out = ImportFile {
            name: file.name.clone(),
            archive: file.archive,
            bytes: file.bytes as f64,
            fbx: u32::try_from(file.entries.len()).unwrap_or(u32::MAX),
            pack: pack.clone(),
            rig: None,
            bones: 0,
            prefix: None,
            fps: None,
            skinned: 0,
            clips: Vec::new(),
            left_out: file.left_out.clone(),
        };
        for entry in &file.entries {
            let Some(read) = by_id.get(entry.id.as_str()) else {
                out.left_out.push(LeftOut {
                    file: entry.file.clone(),
                    reason: "Blender did not read it".to_string(),
                });
                continue;
            };
            if let Some(err) = &read.error {
                out.left_out.push(LeftOut {
                    file: entry.file.clone(),
                    reason: format!("Blender could not read it: {err}"),
                });
                continue;
            }
            let bones: Vec<String> = read.bones.iter().map(|b| b.name.clone()).collect();
            let kind = rig::detect(&bones, false);
            if out.rig.is_none() {
                out.rig = Some(kind);
                out.bones = u32::try_from(bones.len()).unwrap_or(u32::MAX);
                out.prefix = rig::prefix(&bones);
            }
            out.skinned += u32::from(read.skinned);
            let moving: Vec<_> = read.clips.iter().filter(|c| c.has_keys).collect();
            if moving.is_empty() {
                out.left_out.push(LeftOut {
                    file: entry.file.clone(),
                    reason: "it holds one pose and no keys".to_string(),
                });
                continue;
            }
            for (n, clip) in read.clips.iter().enumerate().filter(|(_, c)| c.has_keys) {
                let name = if moving.len() == 1 {
                    names::clip_name(&entry.file)
                } else {
                    clip.name.clone()
                };
                let in_library = existing.iter().find(|c| c.hash == clip.hash);
                let earlier = seen.iter().find(|(h, _)| *h == clip.hash).map(|(_, m)| m);
                let same_name = existing.iter().find(|c| {
                    c.pack.eq_ignore_ascii_case(&pack) && c.name.eq_ignore_ascii_case(&name)
                });
                let (status, matched) = match (in_library, earlier, same_name) {
                    (Some(c), _, _) => (ClipStatus::Duplicate, Some(matched_of(c))),
                    (None, Some(m), _) => (ClipStatus::Duplicate, Some(m.clone())),
                    (None, None, Some(c)) => (ClipStatus::Revision, Some(matched_of(c))),
                    _ => (ClipStatus::New, None),
                };
                seen.push((
                    clip.hash.clone(),
                    MatchedClip {
                        id: None,
                        name: name.clone(),
                        pack: pack.clone(),
                    },
                ));
                out.clips.push(PlanClip {
                    key: format!("{}/{n}", entry.id),
                    tags: names::tags_for(&name),
                    name,
                    file: entry.file.clone(),
                    pack: pack.clone(),
                    rig: kind,
                    bones: u32::try_from(bones.len()).unwrap_or(u32::MAX),
                    frames: clip.frames,
                    fps: clip.fps,
                    length: clip.length,
                    root_motion: clip.root_motion.clone(),
                    loops: clip.loops,
                    bytes: clip.bytes.unwrap_or(0) as f64,
                    status,
                    matched,
                });
            }
        }
        out.fps = common_fps(&out.clips);
        files.push(out);
    }
    summarise(token, files, request)
}

fn matched_of(clip: &AnimationClip) -> MatchedClip {
    MatchedClip {
        id: Some(clip.id.clone()),
        name: clip.name.clone(),
        pack: clip.pack.clone(),
    }
}

/// Whether a clip goes in under `request` (the defaults without one).
pub(crate) fn goes_in(clip: &PlanClip, request: Option<&ImportRequest>) -> bool {
    let named = |list: &[String]| list.iter().any(|k| k == &clip.key);
    if request.is_some_and(|r| named(&r.skip)) {
        return false;
    }
    clip.status != ClipStatus::Duplicate || request.is_some_and(|r| named(&r.keep_both))
}

fn summarise(token: &str, files: Vec<ImportFile>, request: Option<&ImportRequest>) -> ImportPlan {
    let going: Vec<&PlanClip> = files
        .iter()
        .flat_map(|f| &f.clips)
        .filter(|c| goes_in(c, request))
        .collect();
    let mut tags: BTreeMap<String, u32> = BTreeMap::new();
    for tag in going.iter().flat_map(|c| &c.tags) {
        *tags.entry(tag.clone()).or_default() += 1;
    }
    let mut tags: Vec<TagCount> = tags
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
    let left: usize = files.iter().map(|f| f.left_out.len()).sum();
    ImportPlan {
        token: token.to_string(),
        read_bytes: files.iter().map(|f| f.bytes).sum(),
        stored_bytes: going.iter().map(|c| c.bytes).sum(),
        add: u32::try_from(going.len()).unwrap_or(u32::MAX),
        left_out: u32::try_from(left).unwrap_or(u32::MAX),
        tags,
        files,
    }
}
