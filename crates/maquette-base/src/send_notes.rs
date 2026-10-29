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
}
