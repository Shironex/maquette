//! Names and tags for library clips: a clip's name from its file, a pack's
//! from its zip, and tags read from the words of a clip's name.

use std::path::Path;

/// Words in a clip's name and the tag each one gives.
const TAG_WORDS: [(&str, &[&str]); 6] = [
    (
        "combat",
        &[
            "sword", "shield", "slash", "block", "attack", "punch", "kick", "parry", "lunge",
            "stab", "strike", "combo", "guard", "draw", "sheath", "dodge",
        ],
    ),
    ("magic", &["magic", "spell", "cast", "casting", "mage"]),
    (
        "locomotion",
        &[
            "walk", "walking", "run", "running", "jog", "sprint", "strafe", "turn", "jump", "step",
            "crouch", "roll", "rolling",
        ],
    ),
    ("dodge", &["dodge", "evade", "roll", "rolling", "sidestep"]),
    ("idle", &["idle", "breathing", "stand"]),
    (
        "hit react",
        &[
            "hit",
            "impact",
            "react",
            "reaction",
            "stagger",
            "death",
            "dying",
            "knockback",
        ],
    ),
];

fn words(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The tags a clip's name suggests, in the table's order: "Sword And Shield
/// Slash" is combat, "Run To Rolling" locomotion and dodge.
pub fn tags_for(name: &str) -> Vec<String> {
    let found = words(name);
    TAG_WORDS
        .iter()
        .filter(|(_, hits)| found.iter().any(|w| hits.contains(&w.as_str())))
        .map(|(tag, _)| (*tag).to_string())
        .collect()
}

/// A file's name without its folder or extension.
pub fn stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
}

/// A clip's name from its file: `Standing Dodge Left.fbx` stays as it is,
/// `heroine_cast_take3.fbx` reads `heroine cast take3`.
pub fn clip_name(file: &str) -> String {
    let name = stem(file).replace('_', " ");
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        "Clip".to_string()
    } else {
        name
    }
}

/// A pack's name from its zip: `Pro Sword and Shield Pack.zip`.
pub fn pack_name(archive: &str) -> String {
    clip_name(archive)
}

/// Trims a pack name the person typed, or `None` for an empty one.
pub fn clean_pack(pack: &str) -> Option<String> {
    let pack = pack.split_whitespace().collect::<Vec<_>>().join(" ");
    (!pack.is_empty()).then(|| pack.chars().take(80).collect())
}
