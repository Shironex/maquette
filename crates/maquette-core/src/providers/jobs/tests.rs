//! Tests for matching inbox file names to job ids.

use super::records::match_job;

#[test]
fn inbox_names_match_the_longest_job_id() {
    let ids = ["lantern-1", "lantern-1-2", "fox-3"];
    let find = |stem: &str| match_job(stem, ids.iter().copied());
    assert_eq!(find("lantern-1"), Some("lantern-1"));
    assert_eq!(find("Lantern_1"), Some("lantern-1"));
    assert_eq!(find("lantern-1-2"), Some("lantern-1-2"));
    assert_eq!(find("lantern-1-final"), Some("lantern-1"));
    assert_eq!(find("lantern-10"), None);
    assert_eq!(find("fox 3 v2"), Some("fox-3"));
    assert_eq!(find("wolf-1"), None);
}
