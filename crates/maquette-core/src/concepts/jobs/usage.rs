//! Codex's usage limit: "You've hit your usage limit. ... try again at
//! 4:21 PM." Unlike a rate limit it lasts until a set time and holds for
//! every job, so the job that met it and the rest of the queue wait until
//! then and run again on their own.
//!
//! The time is read leniently: "at 4:21 PM", "at 16:21", "at Oct 28th,
//! 4:21 PM", "in 37 minutes", "in 1 hour 5 minutes". Clock times are local
//! time; one already past today means tomorrow.

use chrono::{Datelike, Duration as Span, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

/// Added to the time Codex names, so the rerun lands after the limit lifts.
const MARGIN: Span = Span::minutes(1);
/// A wait longer than this is taken for a misread time.
const LONGEST: Span = Span::days(8);

/// When Codex said the limit lifts, before it is resolved against now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Resume {
    /// "in 37 minutes"
    In(Span),
    /// "at 4:21 PM", or "at Oct 28th, 4:21 PM" with its month and day.
    At {
        date: Option<(u32, u32)>,
        time: NaiveTime,
    },
}

fn normalise(text: &str) -> String {
    text.replace(['\u{2019}', '\u{2018}', '`'], "'")
        .to_lowercase()
}

/// True when `text` says Codex's usage limit was hit.
pub(super) fn says_usage_limited(text: &str) -> bool {
    let lower = normalise(text);
    ["usage limit", "usage_limit", "usage-limit"]
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Reads when the limit lifts from Codex's message, `None` when it names
/// no time this can read.
pub(super) fn parse_resume(text: &str) -> Option<Resume> {
    let lower = normalise(text);
    let from = lower.find("usage limit").unwrap_or(0);
    let at = lower[from..].find("try again")? + from + "try again".len();
    let rest = lower[at..].trim_start();
    if let Some(after) = rest.strip_prefix("in ") {
        return parse_span(after).map(Resume::In);
    }
    let after = rest
        .strip_prefix("at ")
        .or_else(|| rest.strip_prefix("after "))?;
    parse_at(after)
}

/// "37 minutes", "1 hour 5 minutes", "2h 3m", "1 day and 4 hours".
fn parse_span(text: &str) -> Option<Span> {
    let mut total = Span::zero();
    let mut tokens = tokens(text).into_iter().peekable();
    let mut any = false;
    while let Some(token) = tokens.next() {
        if token == "and" {
            continue;
        }
        let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            break;
        }
        let n: i64 = digits.parse().ok()?;
        let unit = match &token[digits.len()..] {
            "" => tokens.next()?,
            glued => glued.to_string(),
        };
        let one = match unit.trim_end_matches('.') {
            "d" | "day" | "days" => Span::days(1),
            "h" | "hr" | "hrs" | "hour" | "hours" => Span::hours(1),
            "m" | "min" | "mins" | "minute" | "minutes" => Span::minutes(1),
            "s" | "sec" | "secs" | "second" | "seconds" => Span::seconds(1),
            _ => break,
        };
        total += one * i32::try_from(n).ok()?;
        any = true;
    }
    (any && total > Span::zero()).then_some(total)
}

/// Words split at spaces and commas, with closing punctuation dropped
/// ("4:21 pm." reads as "4:21 pm", "p.m." as "p.m").
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .map(|t| t.trim_end_matches(['.', ')', ';', '!']).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

fn month(word: &str) -> Option<u32> {
    if word.len() < 3 || !word.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let n = MONTHS.iter().position(|m| word.starts_with(m))?;
    u32::try_from(n + 1).ok()
}

/// "28th", "28" as a day of the month.
fn day(word: &str) -> Option<u32> {
    let digits: String = word.chars().take_while(char::is_ascii_digit).collect();
    let suffix = &word[digits.len()..];
    if !matches!(suffix, "" | "st" | "nd" | "rd" | "th") {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    (1..=31).contains(&n).then_some(n)
}

/// "4:21 pm", "16:21", "Oct 28th, 4:21 PM", "28 Oct at 4:21 PM", "4pm".
fn parse_at(text: &str) -> Option<Resume> {
    let words = tokens(text);
    let mut i = 0;
    let mut date = None;
    let word = |i: usize| words.get(i).map(String::as_str);
    if let (Some(m), Some(d)) = (word(0).and_then(month), word(1).and_then(day)) {
        date = Some((m, d));
        i = 2;
    } else if let (Some(d), Some(m)) = (word(0).and_then(day), word(1).and_then(month)) {
        date = Some((m, d));
        i = 2;
    }
    if date.is_some() && word(i) == Some("at") {
        i += 1;
    }
    // The clock and its am or pm, which may be glued on ("4:21pm").
    let clock = word(i)?;
    let split = clock
        .find(|c: char| !(c.is_ascii_digit() || c == ':' || c == '.'))
        .unwrap_or(clock.len());
    let (digits, glued) = clock.split_at(split);
    let digits = digits.trim_end_matches('.');
    let meridiem = if glued.is_empty() {
        word(i + 1).filter(|w| meridiem(w).is_some())
    } else {
        Some(glued)
    }
    .and_then(meridiem);
    let (hour, minute) = match digits.split_once([':', '.']) {
        Some((h, m)) if m.len() == 2 => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        Some(_) => return None,
        // A bare hour needs its am or pm ("4 pm"); "at 4" says too little.
        None if meridiem.is_some() => (digits.parse::<u32>().ok()?, 0),
        None => return None,
    };
    let hour = match meridiem {
        Some(pm) => {
            if !(1..=12).contains(&hour) {
                return None;
            }
            hour % 12 + if pm { 12 } else { 0 }
        }
        None => hour,
    };
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
    Some(Resume::At { date, time })
}

/// `Some(true)` for pm, `Some(false)` for am.
fn meridiem(word: &str) -> Option<bool> {
    match word.replace('.', "").as_str() {
        "am" => Some(false),
        "pm" => Some(true),
        _ => None,
    }
}

/// When to run again, in local time: the named time plus a minute. A clock
/// time already past means tomorrow; a date already past means next year.
/// `None` when the wait would be far too long to be right.
pub(super) fn resolve(resume: Resume, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let at = match resume {
        Resume::In(span) => now + span,
        Resume::At { date: None, time } => {
            let today = now.date().and_time(time);
            if today < now {
                today + Span::days(1)
            } else {
                today
            }
        }
        Resume::At {
            date: Some((month, day)),
            time,
        } => {
            let this_year = NaiveDate::from_ymd_opt(now.year(), month, day)?.and_time(time);
            if this_year + Span::days(1) < now {
                NaiveDate::from_ymd_opt(now.year() + 1, month, day)?.and_time(time)
            } else {
                this_year.max(now)
            }
        }
    };
    (at - now <= LONGEST).then_some(at + MARGIN)
}

/// Milliseconds since the epoch of a local date and time.
fn local_ms(at: NaiveDateTime) -> Option<f64> {
    // A time a clock change skips is read an hour later.
    let local = Local
        .from_local_datetime(&at)
        .earliest()
        .or_else(|| Local.from_local_datetime(&(at + Span::hours(1))).earliest())?;
    #[expect(clippy::cast_precision_loss, reason = "ms since the epoch fit f64")]
    Some(local.timestamp_millis() as f64)
}

/// When the job runs again after Codex's usage limit message `text`, ms
/// since the epoch, or `None` when the message names no readable time.
pub(super) fn resume_at_ms(text: &str) -> Option<f64> {
    let now = Local::now().naive_local();
    local_ms(resolve(parse_resume(text)?, now)?)
}

/// "16:22" for ms since the epoch, in local time.
pub(super) fn clock(ms: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "ms since the epoch fit i64"
    )]
    Local
        .timestamp_millis_opt(ms as i64)
        .single()
        .map(|t| t.format("%H:%M").to_string())
        .unwrap_or_default()
}

/// The line a job waiting for the usage limit shows.
pub(super) fn waiting_message(until_ms: f64) -> String {
    format!(
        "Waiting for the Codex usage limit until about {}. It runs again on its own then.",
        clock(until_ms)
    )
}

#[cfg(test)]
mod tests {
    use super::super::retry::{Backoff, usage_limited};
    use super::super::{ConceptJob, ConceptJobState};
    use super::*;

    const CODEX: &str = "ERROR: You've hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro), visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at 4:21 PM.";

    fn at(h: u32, m: u32) -> Resume {
        Resume::At {
            date: None,
            time: NaiveTime::from_hms_opt(h, m, 0).expect("time"),
        }
    }

    fn day(y: i32, mo: u32, d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .and_then(|d| d.and_hms_opt(h, m, 0))
            .expect("date")
    }

    #[test]
    fn tells_the_usage_limit_apart_from_other_errors() {
        assert!(says_usage_limited(CODEX));
        assert!(says_usage_limited(
            "ERROR: You\u{2019}ve hit your usage limit. Try again later."
        ));
        assert!(says_usage_limited("{\"type\":\"usage_limit_reached\"}"));
        assert!(!says_usage_limited(
            "ERROR: exceeded retry limit, last status: 429 Too Many Requests"
        ));
        assert!(!says_usage_limited("error: not signed in"));
    }

    #[test]
    fn reads_the_time_in_every_shape_codex_may_print() {
        assert_eq!(parse_resume(CODEX), Some(at(16, 21)));
        let curly = CODEX.replace('\'', "\u{2019}");
        assert_eq!(parse_resume(&curly), Some(at(16, 21)));
        let cases = [
            ("usage limit. Try again at 16:21.", at(16, 21)),
            ("usage limit, try again at 4:21pm", at(16, 21)),
            ("usage limit, try again at 4:21 p.m.", at(16, 21)),
            ("usage limit, try again at 9:05 AM", at(9, 5)),
            ("usage limit, try again at 12:30 AM", at(0, 30)),
            ("usage limit, try again at 12:30 PM", at(12, 30)),
            ("usage limit, try again at 4 PM", at(16, 0)),
            (
                "usage limit. Try again at Oct 28th, 4:21 PM.",
                Resume::At {
                    date: Some((10, 28)),
                    time: NaiveTime::from_hms_opt(16, 21, 0).expect("time"),
                },
            ),
            (
                "usage limit. Try again at 28 October at 16:21",
                Resume::At {
                    date: Some((10, 28)),
                    time: NaiveTime::from_hms_opt(16, 21, 0).expect("time"),
                },
            ),
            (
                "usage limit. Try again in 37 minutes.",
                Resume::In(Span::minutes(37)),
            ),
            (
                "usage limit. Try again in 1 hour and 5 minutes.",
                Resume::In(Span::minutes(65)),
            ),
            (
                "usage limit. Try again in 2h 3m",
                Resume::In(Span::minutes(123)),
            ),
            (
                "usage limit. Try again in 3 days",
                Resume::In(Span::days(3)),
            ),
        ];
        for (text, want) in cases {
            assert_eq!(parse_resume(text), Some(want), "{text}");
        }
    }

    #[test]
    fn names_no_time_when_codex_names_none() {
        for text in [
            "You've hit your usage limit. Try again later.",
            "You've hit your usage limit.",
            "usage limit, try again at 4",
            "usage limit, try again at 25:00",
            "usage limit, try again at 13:00 PM",
            "usage limit, try again in a while",
        ] {
            assert_eq!(parse_resume(text), None, "{text}");
        }
    }

    #[test]
    fn a_clock_time_already_past_means_tomorrow_plus_a_minute() {
        let now = day(2026, 10, 27, 10, 0);
        assert_eq!(resolve(at(16, 21), now), Some(day(2026, 10, 27, 16, 22)));
        assert_eq!(resolve(at(9, 30), now), Some(day(2026, 10, 28, 9, 31)));
        assert_eq!(
            resolve(Resume::In(Span::minutes(37)), now),
            Some(day(2026, 10, 27, 10, 38))
        );
        let dated = |mo, d| Resume::At {
            date: Some((mo, d)),
            time: NaiveTime::from_hms_opt(16, 21, 0).expect("time"),
        };
        assert_eq!(resolve(dated(10, 28), now), Some(day(2026, 10, 28, 16, 22)));
        // New Year's: a date that passed is next year's.
        let eve = day(2026, 12, 31, 23, 0);
        assert_eq!(resolve(dated(1, 1), eve), Some(day(2027, 1, 1, 16, 22)));
        // Far too long is taken for a misread.
        assert_eq!(resolve(Resume::In(Span::days(30)), now), None);
    }

    fn job() -> ConceptJob {
        ConceptJob {
            id: "porch-01-x".into(),
            run_id: Some("run-1".into()),
            project: "moon".into(),
            direction: "porch".into(),
            direction_name: "Porch".into(),
            index: 1,
            file: "concepts/porch/porch_01.png".into(),
            title: None,
            state: ConceptJobState::Failed,
            queued_at_ms: 0.0,
            started_at_ms: Some(0.0),
            finished_at_ms: Some(1.0),
            message: Some(CODEX.into()),
            width: None,
            height: None,
            log_tail: vec![CODEX.into()],
            retry_at_ms: None,
            retries: 0,
            usage_limit: false,
            style_revision: None,
            backend: crate::concepts::backend::ImageBackend::Codex,
            refused: false,
        }
    }

    #[test]
    fn a_usage_limited_job_waits_until_the_named_time_without_spending_a_try() {
        let mut job = job();
        let now = 1_000_000.0;
        let until = now + 37.0 * 60_000.0;
        let wait = usage_limited(&mut job, Some(until), &Backoff::default(), now);
        assert_eq!(wait, Some(std::time::Duration::from_secs(37 * 60)));
        assert_eq!(job.state, ConceptJobState::RateLimited);
        assert!(job.usage_limit);
        assert_eq!(job.retry_at_ms, Some(until));
        assert_eq!(job.retries, 0);
        let message = job.message.expect("message");
        assert_eq!(
            message,
            format!(
                "Waiting for the Codex usage limit until about {}. It runs again on its own then.",
                clock(until)
            )
        );
    }

    #[test]
    fn a_usage_limit_with_no_time_falls_back_to_the_rate_limit_backoff() {
        let mut job = job();
        let wait = usage_limited(&mut job, None, &Backoff::default(), 0.0);
        assert_eq!(wait, Some(std::time::Duration::from_secs(180)));
        assert_eq!(job.state, ConceptJobState::RateLimited);
        assert!(job.usage_limit);
        assert_eq!(job.retries, 1);
        let at = job.retry_at_ms.expect("retry at");
        assert_eq!(job.message, Some(waiting_message(at)));
        // Out of tries, it fails and says so.
        let mut spent = self::job();
        spent.retries = 3;
        assert_eq!(
            usage_limited(&mut spent, None, &Backoff::default(), 0.0),
            None
        );
        assert_eq!(spent.state, ConceptJobState::Failed);
        assert!(!spent.usage_limit);
        assert_eq!(
            spent.message.as_deref(),
            Some("Codex's usage limit is hit, 4 times in a row. Try again later with Retry.")
        );
    }
}
