//! When two names count as the same: lowercased, trimmed, and without a
//! trailing number (`Desk lamp 2`, `desk lamp-2` and `Desk lamp_2` all
//! read as one name with `Desk lamp`).

/// The name as the same-name checks compare it.
pub(super) fn name_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let digits = lower.trim_end_matches(|c: char| c.is_ascii_digit());
    if digits.len() < lower.len()
        && let Some(stem) = digits
            .strip_suffix(' ')
            .or_else(|| digits.strip_suffix('-'))
            .or_else(|| digits.strip_suffix('_'))
        && !stem.trim().is_empty()
    {
        return stem.trim().to_string();
    }
    lower
}

/// Whether `a` and `b` are the same name, a trailing number aside.
pub(super) fn same_name(a: &str, b: &str) -> bool {
    name_key(a) == name_key(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trailing_number_and_case_do_not_count() {
        for name in ["Desk lamp 2", " desk lamp ", "DESK LAMP-12", "desk lamp_3"] {
            assert_eq!(name_key(name), "desk lamp", "{name}");
        }
        assert_eq!(name_key("R2"), "r2");
        assert_eq!(name_key("2"), "2");
        assert!(same_name("Crate", "crate 2"));
        assert!(!same_name("Crate", "Crates"));
    }
}
