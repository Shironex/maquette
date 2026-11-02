//! The one set of characters a canvas trims from the ends of its name,
//! titles, addresses and labels. The view trims the same set
//! (`apps/web/src/state/plan-text.ts`): Rust's `trim` and JavaScript's
//! `trim` differ (U+0085 and U+FEFF), and a canvas both sides trimmed
//! differently would read as changed and save again on every open.

/// White space as Rust's `trim` and JavaScript's `trim` know it, together.
pub const TRIMMED: [char; 26] = [
    '\u{9}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{20}', '\u{85}', '\u{A0}', '\u{1680}',
    '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}',
    '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}',
    '\u{FEFF}',
];

/// `text` without [`TRIMMED`] characters at either end.
pub fn trimmed(text: &str) -> &str {
    text.trim_matches(|c| TRIMMED.contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_white_space_is_trimmed_and_nothing_else() {
        assert_eq!(trimmed("\u{FEFF} porch \u{85}"), "porch");
        assert_eq!(trimmed("\u{3000}\u{2028}a b\t\n"), "a b");
        assert_eq!(
            trimmed("\u{200B}x\u{200B}"),
            "\u{200B}x\u{200B}",
            "zero width space stays"
        );
        assert_eq!(trimmed(""), "");
        for c in TRIMMED {
            assert!(c.is_whitespace() || c == '\u{FEFF}', "{c:?} is white space");
        }
    }
}
