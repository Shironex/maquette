//! What the send behind an inbox model asked for, so a check does not warn
//! about something chosen on purpose. Plain data shared by the inbox checks
//! and the provider sends, which sit side by side above this crate, so it
//! lives here until the checks have a home above both.

/// Which maps the send behind a model asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SentMaps {
    /// Not from a send, or one that asked for the full set.
    #[default]
    Any,
    /// Sent with PBR off: base color only.
    NoPbr,
    /// Sent with texture off: no maps at all.
    NoTexture,
}

/// The provider a send went to, so a check can name the right settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SentTo {
    /// Not from a send.
    #[default]
    Nobody,
    Tripo,
    Meshy,
}

/// The version a re-texture painted, for comparing what came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetextureSource {
    pub version: u32,
    /// How many mesh objects draw it.
    pub objects: u32,
    pub triangles: u32,
}

/// What the send record says about a model, so a check does not warn
/// about something that was chosen on purpose.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendNotes {
    pub maps: SentMaps,
    /// Sent with a texture quality above the standard one.
    pub large_textures: bool,
    /// Made by hand on Tripo's site, where a Smart Mesh character exports
    /// base color only whatever the PBR setting.
    pub by_hand_tripo: bool,
    pub sent_to: SentTo,
    /// Made on the provider's own site, not over its API.
    pub by_hand: bool,
    /// A split into parts, which has no PBR setting: Meshy's comes back
    /// grey on purpose, the texture follows with a bake back or a re-texture.
    pub split: bool,
    /// A re-texture, which has no PBR setting either: it always asks for it.
    pub retexture_of: Option<RetextureSource>,
}
