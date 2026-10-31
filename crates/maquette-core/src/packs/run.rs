//! A pack's thread: runs the queued models one at a time and settles,
//! stops or zips the pack at the end.

use std::path::Path;

use super::decide::stopped_at;
use super::finish::ZipChoice;
use super::job::{Drive, Inner, zip_now};
use super::model_run::{ModelRun, run_one};
use super::record::{PackItem, PackItemState, PackRecord, PackState};
use super::{EXPORTS_DIR, RECORDS_DIR};

/// After the last model: zips at once when everything passes, else waits.
pub(super) fn settle(inner: &Inner, drive: &Drive, record: &mut PackRecord) {
    if record.items.iter().all(|i| i.state.passes()) {
        let _ = zip_now(inner, &drive.root, record, ZipChoice::Passing);
    } else {
        record.state = PackState::Waiting;
        inner.tell(&drive.root, record);
    }
}

enum Next {
    Model(usize, Box<PackItem>),
    Zip(ZipChoice),
    Stop,
    Settle,
}

pub(super) fn run_pack(inner: &Inner, drive: &Drive, id: &str) {
    let pictures = drive.root.join(EXPORTS_DIR).join(RECORDS_DIR).join(id);
    loop {
        let (next, record) = {
            let mut active = inner.active();
            let Some(a) = active.get_mut(id) else {
                return;
            };
            let next = if let Some(choice) = a.zip {
                Next::Zip(choice)
            } else if let Some(at) = a.record.items.iter().position(|i| {
                i.state == PackItemState::Queued
                    && a.only.as_ref().is_none_or(|key| *key == i.model.key)
            }) {
                if a.stop {
                    Next::Stop
                } else {
                    a.record.items[at].state = PackItemState::Running;
                    Next::Model(at, Box::new(a.record.items[at].clone()))
                }
            } else if a.only.is_some()
                && a.record
                    .items
                    .iter()
                    .any(|i| i.state == PackItemState::Queued)
            {
                Next::Stop
            } else {
                Next::Settle
            };
            if !matches!(next, Next::Model(..)) {
                a.closing = true;
            }
            (next, a.record.clone())
        };
        let mut record = record;
        match next {
            Next::Model(at, item) => {
                inner.tell(&drive.root, &record);
                let run = ModelRun {
                    runner: &inner.runner,
                    tools: &drive.tools,
                    dir: Path::new(&record.dir),
                    pictures: &pictures,
                    settings: &record.settings,
                    defaults: drive.defaults,
                };
                let done = run_one(&run, &item);
                let snapshot = {
                    let mut active = inner.active();
                    let Some(a) = active.get_mut(id) else {
                        return;
                    };
                    a.record.items[at] = done;
                    a.record.clone()
                };
                inner.tell(&drive.root, &snapshot);
                continue;
            }
            Next::Zip(choice) => {
                for item in &mut record.items {
                    if !item.state.is_settled() {
                        item.state = PackItemState::Queued;
                    }
                }
                let _ = zip_now(inner, &drive.root, &mut record, choice);
            }
            Next::Stop => {
                record.state = PackState::Stopped;
                record.message = Some(stopped_at(&record, "Stopped"));
                inner.tell(&drive.root, &record);
            }
            Next::Settle => settle(inner, drive, &mut record),
        }
        // Only now, with the last state saved, does the pack stop running.
        inner.active().remove(id);
        return;
    }
}
