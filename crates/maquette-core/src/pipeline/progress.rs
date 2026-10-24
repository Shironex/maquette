//! How far a running step is, read from its output. Tools report progress
//! in a few shapes, and the runner turns them into one number from 0 to 1:
//!
//! - `progress N/M` or `progress P%`, after an optional `[tag] ` prefix and
//!   in any case: the starter scripts print this (`[bake_light] progress
//!   2/5 crate`, meaning 2 of 5 objects are done).
//! - Cycles' `Sample N/M`, which Blender prints while it renders or bakes.
//!   Inside a script's `progress N/M` it fills the slice of the current
//!   item, so a bake of 5 objects moves smoothly through each one.
//! - A step made of several program calls (Compress runs one chain per
//!   model) counts each finished call.
//!
//! A step that reports none of these has no progress, and the UI draws an
//! indeterminate bar.

/// A fraction from a line, if the line reports one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reported {
    /// `progress N/M` or `progress P%`: how much of the whole step is done.
    Step(f64),
    /// `Sample N/M`: how far the current render or bake is.
    Sample(f64),
}

fn fraction(text: &str) -> Option<f64> {
    let text = text.trim_start_matches([':', ' ', '\t']);
    let token: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '/' | '%'))
        .collect();
    if let Some(percent) = token.strip_suffix('%') {
        let value: f64 = percent.parse().ok()?;
        return value.is_finite().then(|| (value / 100.0).clamp(0.0, 1.0));
    }
    let (done, total) = token.split_once('/')?;
    let done: f64 = done.parse().ok()?;
    let total: f64 = total.parse().ok()?;
    (total > 0.0 && done.is_finite()).then(|| (done / total).clamp(0.0, 1.0))
}

/// Reads one output line.
pub fn parse(line: &str) -> Option<Reported> {
    let mut rest = line.trim();
    // An optional `[tag] ` prefix, as the starter scripts print.
    if rest.starts_with('[')
        && let Some(end) = rest.find(']')
    {
        rest = rest[end + 1..].trim_start();
    }
    let lower = rest.to_ascii_lowercase();
    if let Some(after) = lower.strip_prefix("progress") {
        return fraction(after).map(Reported::Step);
    }
    // Cycles prints its samples at the end of a status line:
    // `Fra:1 Mem:12M | Time:00:01.20 | ... | Sample 8/16`.
    let at = lower.rfind("sample ")?;
    let before = lower[..at].trim_end();
    if !(before.is_empty() || before.ends_with('|')) {
        return None;
    }
    fraction(&lower[at + "sample ".len()..]).map(Reported::Sample)
}

/// Folds reported lines and finished calls into one fraction.
#[derive(Debug, Clone, Default)]
pub struct Tracker {
    /// The program call running now and how many the step makes.
    call: (usize, usize),
    /// The last `progress` a script printed.
    step: Option<f64>,
    /// The share of one item, `1 / M`, from the last `progress N/M`.
    slice: Option<f64>,
    sample: Option<f64>,
    seen: bool,
}

impl Tracker {
    pub fn new(calls: usize) -> Self {
        Self {
            call: (0, calls.max(1)),
            ..Self::default()
        }
    }

    /// The step moved on to call `index` (0 based).
    pub fn start_call(&mut self, index: usize) {
        self.call.0 = index;
        self.step = None;
        self.slice = None;
        self.sample = None;
        if self.call.1 > 1 {
            self.seen = true;
        }
    }

    /// Takes one output line. Returns true when the fraction changed.
    pub fn line(&mut self, line: &str) -> bool {
        let before = self.value();
        match parse(line) {
            Some(Reported::Step(value)) => {
                self.step = Some(value);
                self.slice = slice_of(line);
                self.sample = None;
                self.seen = true;
            }
            Some(Reported::Sample(value)) => {
                self.sample = Some(value);
                self.seen = true;
            }
            None => return false,
        }
        self.value() != before
    }

    /// How far the whole step is, or `None` when nothing reported yet.
    pub fn value(&self) -> Option<f64> {
        if !self.seen {
            return None;
        }
        let inner = match (self.step, self.sample) {
            (Some(step), Some(sample)) => {
                let slice = self.slice.unwrap_or(0.0);
                (step + sample * slice).min(1.0)
            }
            (Some(step), None) => step,
            (None, Some(sample)) => sample,
            (None, None) => 0.0,
        };
        #[expect(clippy::cast_precision_loss, reason = "call counts are tiny")]
        let (index, calls) = (self.call.0 as f64, self.call.1 as f64);
        Some(((index + inner) / calls).clamp(0.0, 1.0))
    }
}

/// `1 / M` for a `progress N/M` line, so samples can fill the current item.
fn slice_of(line: &str) -> Option<f64> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find("progress")?;
    let rest = lower[at + "progress".len()..].trim_start_matches([':', ' ', '\t']);
    let token: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '/')
        .collect();
    let (_, total) = token.split_once('/')?;
    let total: f64 = total.parse().ok()?;
    (total > 0.0).then(|| 1.0 / total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-9)
    }

    #[test]
    fn reads_the_shapes_tools_print() {
        assert_eq!(
            parse("[bake_light] progress 2/5 crate"),
            Some(Reported::Step(0.4))
        );
        assert_eq!(parse("Progress: 50%"), Some(Reported::Step(0.5)));
        assert_eq!(parse("PROGRESS 3/3"), Some(Reported::Step(1.0)));
        assert_eq!(
            parse(
                "Fra:1 Mem:31.20M (Peak 31.20M) | Time:00:00.72 | Mem:2.44M | Scene, ViewLayer | Sample 8/16"
            ),
            Some(Reported::Sample(0.5))
        );
        assert_eq!(parse("Sample 4/16"), Some(Reported::Sample(0.25)));
        // Lines that only mention the words are not progress.
        assert_eq!(parse("[build_scene] imported progress_bar.glb"), None);
        assert_eq!(parse("wrote sample 3/4 of the docs"), None);
        assert_eq!(parse("progress 1/0"), None);
        assert_eq!(parse("Blender 5.2.2"), None);
    }

    #[test]
    fn samples_fill_the_current_item() {
        let mut t = Tracker::new(1);
        assert_eq!(t.value(), None);
        assert!(t.line("[bake_light] progress 1/4 rock"));
        assert!(close(t.value(), 0.25));
        assert!(t.line("Fra:1 | Sample 8/16"));
        assert!(close(t.value(), 0.25 + 0.5 * 0.25));
        assert!(!t.line("Saved \"scene_baked.blend\""));
        assert!(t.line("[bake_light] progress 4/4"));
        assert!(close(t.value(), 1.0));
    }

    #[test]
    fn calls_count_for_multi_call_steps() {
        let mut t = Tracker::new(4);
        t.start_call(0);
        assert!(close(t.value(), 0.0));
        t.start_call(2);
        assert!(close(t.value(), 0.5));
        t.line("progress 50%");
        assert!(close(t.value(), 0.625));
        // A single call step stays indeterminate until a line reports.
        let mut single = Tracker::new(1);
        single.start_call(0);
        assert_eq!(single.value(), None);
    }
}
