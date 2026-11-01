//! Project ids from project names.

const SLUG_MAX_CHARS: usize = 60;

/// Names Windows keeps for devices, with or without an extension. A folder
/// or file with one of them fails or misbehaves there.
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// `"Moon Island"` becomes `"moon-island"`. Common accented Latin letters are
/// folded to ASCII, anything else is dropped. Never empty, never a name
/// Windows reserves (`con` becomes `con-1`).
pub fn slugify(name: &str) -> String {
    latin_slug(name).unwrap_or_else(|| "project".to_string())
}

/// [`slugify`], or `None` when the name has nothing a slug keeps, for
/// example one in Japanese: `slugify` then says `project` for all of them.
pub fn latin_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in name.chars() {
        let folded = fold_char(c);
        if folded.is_empty() {
            if c.is_whitespace() || c.is_ascii_punctuation() {
                pending_dash = true;
            }
            continue;
        }
        if pending_dash && !slug.is_empty() {
            slug.push('-');
        }
        pending_dash = false;
        slug.push_str(&folded);
    }
    let mut slug: String = slug.chars().take(SLUG_MAX_CHARS).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else if WINDOWS_RESERVED.contains(&slug.as_str()) {
        Some(format!("{slug}-1"))
    } else {
        Some(slug)
    }
}

fn fold_char(c: char) -> String {
    if c.is_ascii_alphanumeric() {
        return c.to_ascii_lowercase().to_string();
    }
    let folded = match c.to_lowercase().next().unwrap_or(c) {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ą' | 'ā' | 'ă' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'č' | 'ĉ' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ę' | 'ē' | 'ě' => "e",
        'ğ' | 'ĝ' => "g",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ı' => "i",
        'ł' | 'ľ' | 'ĺ' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'œ' => "oe",
        'ř' | 'ŕ' => "r",
        'ś' | 'š' | 'ş' | 'ŝ' => "s",
        'ß' => "ss",
        'ť' | 'ţ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => "",
    };
    folded.to_string()
}

#[cfg(test)]
mod reserved_tests {
    use super::*;

    #[test]
    fn a_name_windows_reserves_never_becomes_a_slug() {
        for name in ["con", "CON", "Aux", "nul", "PRN", "com1", "LPT9"] {
            let slug = slugify(name);
            assert_eq!(slug, format!("{}-1", name.to_lowercase()), "{name}");
            assert!(!WINDOWS_RESERVED.contains(&slug.as_str()));
        }
        assert_eq!(slugify("console"), "console");
        assert_eq!(slugify("com10"), "com10");
    }

    #[test]
    fn a_name_with_nothing_to_keep_has_no_latin_slug() {
        assert_eq!(latin_slug("\u{96ea}"), None);
        assert_eq!(latin_slug("!!!"), None);
        assert_eq!(slugify("\u{96ea}"), "project");
        assert_eq!(latin_slug("Moon"), Some("moon".to_string()));
    }
}
