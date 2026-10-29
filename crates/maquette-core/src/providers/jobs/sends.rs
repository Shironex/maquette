//! The provider jobs as the sends an asset remove looks at and stops.

use super::{JobState, ProviderJob, ProviderJobs};
use crate::assets::remove::{Sends, Sent, SentState};
use crate::error::CoreResult;

impl From<&ProviderJob> for Sent {
    fn from(job: &ProviderJob) -> Self {
        Self {
            id: job.id.clone(),
            asset: job.asset.clone(),
            images: job.images.clone(),
            provider: job.provider.label(),
            state: match job.state {
                JobState::Running => SentState::Running,
                JobState::Approval | JobState::Queued | JobState::Waiting => SentState::Waiting,
                JobState::Inbox | JobState::Accepted | JobState::Failed | JobState::Cancelled => {
                    SentState::Done
                }
            },
        }
    }
}

impl Sends for ProviderJobs {
    fn jobs(&self, project: &str) -> Vec<Sent> {
        self.list(Some(project)).iter().map(Sent::from).collect()
    }

    fn stop(&self, id: &str) -> CoreResult<()> {
        self.cancel(id).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn job(state: JobState) -> ProviderJob {
        serde_json::from_value(json!({
            "id": "clay-bowl-1",
            "project": "moon-island",
            "asset": "clay-bowl",
            "assetName": "Clay bowl",
            "provider": "by-hand",
            "service": "tripo",
            "variant": "prop",
            "images": ["refs/front.png"],
            "state": state,
            "progress": 0.0,
            "createdAtMs": 0.0,
            "viaAgent": false,
            "cancel": "none",
        }))
        .expect("job")
    }

    #[test]
    fn a_running_job_waits_a_waiting_one_stops_and_the_rest_are_done() {
        let sent = Sent::from(&job(JobState::Running));
        assert_eq!(sent.id, "clay-bowl-1");
        assert_eq!(sent.asset, "clay-bowl");
        assert_eq!(sent.images, ["refs/front.png"]);
        assert_eq!(sent.provider, "By hand");
        assert_eq!(sent.state, SentState::Running);
        for state in [JobState::Approval, JobState::Queued, JobState::Waiting] {
            assert_eq!(
                Sent::from(&job(state)).state,
                SentState::Waiting,
                "{state:?}"
            );
        }
        for state in [
            JobState::Inbox,
            JobState::Accepted,
            JobState::Failed,
            JobState::Cancelled,
        ] {
            assert_eq!(Sent::from(&job(state)).state, SentState::Done, "{state:?}");
        }
    }
}
