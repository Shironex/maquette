//! Page preview approvals: the first start of the dev server of a repo an
//! agent set waits here for a person. One card per project at a time, and
//! a refusal holds for the same question until the app quits or a person
//! asks again from the app, so an agent's retries cannot nag.

use maquette_core::pipeline::history::now_ms;
use maquette_core::site::preview::PreviewAsk;

use super::{Approval, ApprovalKind, Hub};

impl Hub {
    /// The approval `project`'s dev server waits for, if any.
    pub fn preview_for(&self, project: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| a.kind == ApprovalKind::PagePreview && a.project == project)
            .cloned()
    }

    /// Puts `ask` to a person and returns the approval's id, and whether this
    /// call raised it. The same question waiting is that card; a changed one
    /// takes the old card's place, since no one should approve what will not
    /// run.
    pub fn request_preview(&self, project: &str, ask: &PreviewAsk) -> (String, bool) {
        let id = {
            let mut inner = self.lock();
            let waiting = inner
                .approvals
                .iter()
                .find(|a| a.kind == ApprovalKind::PagePreview && a.project == project)
                .map(|a| (a.id.clone(), a.preview.as_ref() == Some(ask)));
            match waiting {
                Some((id, true)) => return (id, false),
                Some((old, false)) => {
                    inner.approvals.retain(|a| a.id != old);
                    inner.decisions.insert(old.clone(), false);
                    inner.preview_withdrawn.insert(old);
                    self.decided.notify_all();
                }
                None => {}
            }
            inner.next_approval += 1;
            let id = format!("approval-{}", inner.next_approval);
            inner.approvals.push(Approval {
                id: id.clone(),
                kind: ApprovalKind::PagePreview,
                project: project.to_string(),
                run_id: None,
                run_number: None,
                step: None,
                job: None,
                asset_name: None,
                service: None,
                detail: None,
                items: Vec::new(),
                requested_at_ms: now_ms(),
                estimate: None,
                concept_jobs: Vec::new(),
                estimate_usd: None,
                asset: None,
                version: None,
                sheets: None,
                preview: Some(ask.clone()),
            });
            id
        };
        (self.on_change)();
        (id, true)
    }

    /// True once when the approval was taken back rather than answered.
    pub(crate) fn take_preview_withdrawn(&self, id: &str) -> bool {
        self.lock().preview_withdrawn.remove(id)
    }

    /// Remembers a person's no to `ask`.
    pub fn refuse_preview(&self, project: &str, ask: &PreviewAsk) {
        self.lock()
            .preview_refused
            .insert(project.to_string(), ask.clone());
    }

    /// True when a person refused this very question.
    pub fn preview_refused(&self, project: &str, ask: &PreviewAsk) -> bool {
        self.lock().preview_refused.get(project) == Some(ask)
    }

    /// A person asked from the app: the earlier no no longer stands.
    pub fn forget_preview_refusal(&self, project: &str) {
        self.lock().preview_refused.remove(project);
    }
}

#[cfg(test)]
mod tests {
    use maquette_core::site::preview::CommandSource;

    use super::*;

    fn ask(command: &str) -> PreviewAsk {
        PreviewAsk {
            repo: "/code/site".to_string(),
            command: command.to_string(),
            source: CommandSource::SiteConfig,
            script_runs: None,
            hooks: Vec::new(),
            script_shell: None,
            files_hash: None,
        }
    }

    #[test]
    fn the_same_question_is_one_card_and_a_new_one_replaces_it() {
        let hub = Hub::new(|| {});
        let (first, raised) = hub.request_preview("p", &ask("pnpm dev"));
        assert!(raised);
        assert_eq!(
            hub.request_preview("p", &ask("pnpm dev")),
            (first.clone(), false)
        );
        let (second, raised) = hub.request_preview("p", &ask("node evil.js"));
        assert!(raised);
        assert_ne!(first, second);
        assert_eq!(hub.approvals().len(), 1);
        assert_eq!(hub.preview_for("p").expect("card").id, second);
        // The thread on the old card wakes up told it was taken back.
        assert_eq!(hub.wait_until(&first, || false), Some(false));
        assert!(hub.take_preview_withdrawn(&first));
        assert!(!hub.take_preview_withdrawn(&second));
    }

    #[test]
    fn a_refusal_holds_for_that_question_only() {
        let hub = Hub::new(|| {});
        hub.refuse_preview("p", &ask("pnpm dev"));
        assert!(hub.preview_refused("p", &ask("pnpm dev")));
        assert!(!hub.preview_refused("p", &ask("pnpm start")));
        assert!(!hub.preview_refused("q", &ask("pnpm dev")));
        hub.forget_preview_refusal("p");
        assert!(!hub.preview_refused("p", &ask("pnpm dev")));
    }
}
