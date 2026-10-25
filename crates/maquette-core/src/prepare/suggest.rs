//! A first guess at an asset's real size from its name and kind: a small
//! table of everyday objects. Only ever shown as a suggestion the person
//! confirms; nothing applies it on its own.

use serde::{Deserialize, Serialize};

use super::{RealSize, SizeAxis, SizeUnit};
use crate::assets::AssetKind;

/// A size from the table and the word of the name it matched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub size: RealSize,
    /// "desk lamp", or "character" for the kind.
    pub matched: String,
}

const H: SizeAxis = SizeAxis::Height;
const W: SizeAxis = SizeAxis::Width;
const D: SizeAxis = SizeAxis::Depth;

/// Name words to a typical size in centimetres. Longer names come first,
/// so "desk lamp" wins over "lamp" and "desk".
const TABLE: &[(&str, SizeAxis, f32)] = &[
    ("coffee table", H, 45.0),
    ("desk lamp", H, 45.0),
    ("floor lamp", H, 160.0),
    ("street lamp", H, 450.0),
    ("street light", H, 450.0),
    ("monitor", W, 55.0),
    ("keyboard", W, 44.0),
    ("headphones", H, 20.0),
    ("notebook", H, 21.0),
    ("bookshelf", H, 180.0),
    ("bookcase", H, 180.0),
    ("lantern", H, 35.0),
    ("laptop", W, 32.0),
    ("teapot", H, 15.0),
    ("bottle", H, 25.0),
    ("candle", H, 12.0),
    ("camera", W, 13.0),
    ("pencil", W, 18.0),
    ("tablet", W, 25.0),
    ("speaker", H, 20.0),
    ("cushion", W, 45.0),
    ("pillow", W, 50.0),
    ("basket", W, 40.0),
    ("bucket", H, 30.0),
    ("barrel", H, 90.0),
    ("bench", W, 120.0),
    ("chair", H, 90.0),
    ("stool", H, 65.0),
    ("table", H, 75.0),
    ("desk", H, 75.0),
    ("sofa", W, 200.0),
    ("couch", W, 200.0),
    ("bed", D, 200.0),
    ("door", H, 210.0),
    ("window", H, 120.0),
    ("fence", H, 100.0),
    ("crate", H, 50.0),
    ("chest", W, 80.0),
    ("vase", H, 30.0),
    ("plant", H, 50.0),
    ("cactus", H, 30.0),
    ("tree", H, 500.0),
    ("bush", H, 100.0),
    ("rock", W, 60.0),
    ("lamp", H, 45.0),
    ("mug", H, 10.0),
    ("cup", H, 9.0),
    ("glass", H, 12.0),
    ("plate", W, 26.0),
    ("bowl", W, 16.0),
    ("book", H, 23.0),
    ("phone", H, 15.0),
    ("mouse", D, 11.0),
    ("clock", W, 30.0),
    ("pot", H, 20.0),
    ("car", D, 450.0),
    ("bike", D, 175.0),
];

/// The words of `name`, lowercase, separated by single spaces.
fn words(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// True when `phrase` is in `text` as whole words, with a plural s allowed.
fn has_phrase(text: &str, phrase: &str) -> bool {
    let padded = format!(" {text} ");
    padded.contains(&format!(" {phrase} ")) || padded.contains(&format!(" {phrase}s "))
}

/// A suggested real size for an asset called `name` of `kind`: a prop
/// from the table above, a character at 170 cm whatever its name (a
/// "lantern keeper" is no lantern). Scenes get none.
pub fn suggest(name: &str, kind: AssetKind) -> Option<Suggestion> {
    let (matched, axis, cm) = match kind {
        AssetKind::Scene => return None,
        AssetKind::Character => ("character".to_string(), SizeAxis::Height, 170.0),
        AssetKind::Prop => {
            let text = words(name);
            let mut table: Vec<&(&str, SizeAxis, f32)> = TABLE.iter().collect();
            table.sort_by_key(|(word, _, _)| std::cmp::Reverse(word.len()));
            let (word, axis, cm) = table
                .into_iter()
                .find(|(word, _, _)| has_phrase(&text, word))?;
            ((*word).to_string(), *axis, *cm)
        }
    };
    let size = if cm >= 100.0 {
        RealSize {
            axis,
            value: cm / 100.0,
            unit: SizeUnit::M,
        }
    } else {
        RealSize {
            axis,
            value: cm,
            unit: SizeUnit::Cm,
        }
    };
    Some(Suggestion { size, matched })
}
