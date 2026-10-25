//! Which waiting By hand send a new download is for. The export name
//! (`maq_<asset>_v<n>_<provider>`) decides first; after that a file that
//! came within a few hours of its handoff goes to the only send waiting
//! then, or to the one its name clearly fits. Anything else is asked.

use crate::providers::handback::{SLACK_MS, name_fit};
use crate::providers::jobs::ProviderJob;
use crate::providers::naming;

/// How long after a handoff a download still counts as its model without
/// a name that says so.
pub(super) const WINDOW_MS: f64 = 6.0 * 3_600_000.0;
/// The least name fit that picks one send out of several.
const FIT: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Verdict {
    /// Take it for this job.
    Job { id: String, reason: String },
    /// Ask which asset it is.
    Ask { reason: String },
}

fn job(id: &str, reason: String) -> Verdict {
    Verdict::Job {
        id: id.to_string(),
        reason,
    }
}

fn stem(name: &str) -> &str {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.rsplit_once('.').map_or(base, |(s, _)| s)
}

/// Decides for a file `name` (and the GLB inside, for a zip) changed at
/// `modified_ms`, given the By hand sends waiting for a model.
pub(super) fn decide(
    name: &str,
    inner: Option<&str>,
    modified_ms: f64,
    waiting: &[ProviderJob],
) -> Verdict {
    let named = naming::parse(name).or_else(|| inner.and_then(naming::parse));
    if let Some(named) = named {
        let id = named.job_id();
        return match waiting
            .iter()
            .find(|j| j.id == id && j.service == named.provider)
        {
            Some(found) => job(&found.id, format!("Named for {}", found.asset_name)),
            None => Verdict::Ask {
                reason: format!(
                    "Named for {} v{} on {}, which is not waiting for a model",
                    named.asset,
                    named.version,
                    named.provider.label()
                ),
            },
        };
    }
    let stems = [Some(stem(name)), inner.map(stem)];
    if let Some(found) = waiting.iter().find(|j| {
        stems
            .iter()
            .flatten()
            .any(|s| s.eq_ignore_ascii_case(&j.id))
    }) {
        return job(&found.id, format!("Named for {}", found.asset_name));
    }
    let recent: Vec<&ProviderJob> = waiting
        .iter()
        .filter(|j| {
            modified_ms + SLACK_MS >= j.created_at_ms && modified_ms - j.created_at_ms <= WINDOW_MS
        })
        .collect();
    let mut fits: Vec<(f64, &ProviderJob)> = recent
        .iter()
        .map(|j| (name_fit(name, inner, j), *j))
        .collect();
    fits.sort_by(|a, b| b.0.total_cmp(&a.0));
    match (recent.as_slice(), fits.as_slice()) {
        // A stale send still waiting makes "the only one" a guess.
        ([only], _) if waiting.len() == 1 => job(
            &only.id,
            format!("The only send waiting, for {}", only.asset_name),
        ),
        (_, [(best, found), rest @ ..])
            if *best >= FIT && rest.first().is_none_or(|(next, _)| next < best) =>
        {
            job(&found.id, format!("Name close to {}", found.asset_name))
        }
        ([], _) => Verdict::Ask {
            reason: "It came long after the waiting handoffs".to_string(),
        },
        _ => Verdict::Ask {
            reason: format!("{} sends are waiting", waiting.len()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::jobs::{CancelMode, JobState, Provider};
    use crate::providers::presets::{PresetVariant, ProviderId};

    const T: f64 = 1_800_000_000_000.0;
    const MIN: f64 = 60_000.0;

    fn waiting(asset: &str, name: &str, n: u32, created: f64) -> ProviderJob {
        ProviderJob {
            id: format!("{asset}-{n}"),
            project: "moon".into(),
            asset: asset.into(),
            asset_name: name.into(),
            provider: Provider::ByHand,
            service: ProviderId::Tripo,
            variant: PresetVariant::Prop,
            images: vec![],
            tripo: None,
            meshy: None,
            state: JobState::Waiting,
            phase: None,
            progress: 0.0,
            remote_id: None,
            credits: None,
            created_at_ms: created,
            started_at_ms: None,
            finished_at_ms: None,
            message: None,
            inbox_file: None,
            handoff: None,
            card: None,
            version: None,
            via_agent: false,
            cancel: CancelMode::StopWaiting,
            batch: None,
            prompt: None,
            redo_of: None,
            estimate: None,
        }
    }

    fn id(v: &Verdict) -> Option<&str> {
        match v {
            Verdict::Job { id, .. } => Some(id),
            Verdict::Ask { .. } => None,
        }
    }

    #[test]
    fn the_export_name_wins_over_time_and_other_names() {
        let jobs = [
            waiting("desk-lamp", "Desk lamp", 2, T),
            waiting("crate", "Crate", 1, T + MIN),
        ];
        let v = decide("maq_desk-lamp_v2_tripo (1).glb", None, T + 2.0 * MIN, &jobs);
        assert_eq!(id(&v), Some("desk-lamp-2"));
        // Even a day later, and inside a zip.
        let v = decide(
            "download.zip",
            Some("maq_crate_v1_tripo.glb"),
            T + 30.0 * 60.0 * MIN,
            &jobs,
        );
        assert_eq!(id(&v), Some("crate-1"));
        // Named for a send that is not waiting: asked, never guessed.
        let v = decide("maq_desk-lamp_v1_tripo.glb", None, T + 2.0 * MIN, &jobs);
        assert!(matches!(v, Verdict::Ask { .. }), "{v:?}");
        // Named for the right send on the other provider: asked too.
        let v = decide("maq_crate_v1_meshy.glb", None, T + 2.0 * MIN, &jobs);
        assert!(matches!(v, Verdict::Ask { .. }), "{v:?}");
        // The old handoff name still works.
        let v = decide("Crate-1.glb", None, T + 2.0 * MIN, &jobs);
        assert_eq!(id(&v), Some("crate-1"));
    }

    #[test]
    fn a_stale_send_still_waiting_means_asking() {
        let jobs = [
            waiting("desk-lamp", "Desk lamp", 1, T),
            waiting("crate", "Crate", 1, T + 8.0 * 60.0 * MIN),
        ];
        let v = decide("model (3).glb", None, T + 8.0 * 60.0 * MIN + MIN, &jobs);
        let Verdict::Ask { reason } = v else {
            panic!("guessed {v:?}");
        };
        assert_eq!(reason, "2 sends are waiting");
    }

    #[test]
    fn without_a_name_the_only_recent_send_takes_it() {
        let jobs = [waiting("desk-lamp", "Desk lamp", 1, T)];
        let v = decide("model (3).glb", None, T + 10.0 * MIN, &jobs);
        assert_eq!(id(&v), Some("desk-lamp-1"));
        // Before the handoff (beyond the clock slack) or hours after: asked.
        assert!(matches!(
            decide("model (3).glb", None, T - 10.0 * MIN, &jobs),
            Verdict::Ask { .. }
        ));
        assert!(matches!(
            decide("model (3).glb", None, T + WINDOW_MS + MIN, &jobs),
            Verdict::Ask { .. }
        ));
    }

    #[test]
    fn several_waiting_sends_need_a_clear_name_or_a_question() {
        let jobs = [
            waiting("desk-lamp", "Desk lamp", 1, T),
            waiting("brush-cup", "Brush cup", 1, T),
        ];
        let v = decide("tripo_desk_lamp_8c1f.glb", None, T + MIN, &jobs);
        assert_eq!(id(&v), Some("desk-lamp-1"));
        let v = decide("model.glb", None, T + MIN, &jobs);
        assert_eq!(
            v,
            Verdict::Ask {
                reason: "2 sends are waiting".into()
            }
        );
        // A name that fits both equally picks neither.
        let jobs = [
            waiting("lamp-a", "Lamp", 1, T),
            waiting("lamp-b", "Lamp", 1, T),
        ];
        assert!(matches!(
            decide("lamp.glb", None, T + MIN, &jobs),
            Verdict::Ask { .. }
        ));
    }
}
