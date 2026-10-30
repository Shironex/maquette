//! The avatar defaults in `maquette.json`: who makes this workspace's VRM
//! avatars and the licence they go out under, filled into every new avatar
//! spec. The licence fields start at the VRM 1.0 defaults, the strictest.

use serde::{Deserialize, Serialize};

use crate::avatar::AvatarSpec;
use crate::error::CoreResult;
use maquette_glb::vrm::{
    AvatarPermission, CommercialUsage, CreditNotation, MaterialMode, Modification, VrmMeta,
};

/// What a new avatar spec starts from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarDefaults {
    /// The author every avatar names first; empty leaves it to fill in.
    pub author: String,
    pub contact_information: Option<String>,
    pub copyright_information: Option<String>,
    pub avatar_permission: AvatarPermission,
    pub commercial_usage: CommercialUsage,
    pub credit_notation: CreditNotation,
    pub modification: Modification,
    pub allow_redistribution: bool,
    pub allow_excessively_violent_usage: bool,
    pub allow_excessively_sexual_usage: bool,
    pub allow_political_or_religious_usage: bool,
    pub allow_antisocial_or_hate_usage: bool,
    pub other_license_url: Option<String>,
    pub material: MaterialMode,
}

impl AvatarDefaults {
    /// A new avatar spec named `name`, with these defaults in its meta.
    pub fn spec(&self, name: &str) -> AvatarSpec {
        let author = self.author.trim();
        AvatarSpec {
            meta: VrmMeta {
                name: name.trim().to_string(),
                authors: if author.is_empty() {
                    Vec::new()
                } else {
                    vec![author.to_string()]
                },
                contact_information: self.contact_information.clone(),
                copyright_information: self.copyright_information.clone(),
                avatar_permission: self.avatar_permission,
                commercial_usage: self.commercial_usage,
                credit_notation: self.credit_notation,
                modification: self.modification,
                allow_redistribution: self.allow_redistribution,
                allow_excessively_violent_usage: self.allow_excessively_violent_usage,
                allow_excessively_sexual_usage: self.allow_excessively_sexual_usage,
                allow_political_or_religious_usage: self.allow_political_or_religious_usage,
                allow_antisocial_or_hate_usage: self.allow_antisocial_or_hate_usage,
                other_license_url: self.other_license_url.clone(),
                ..VrmMeta::default()
            },
            bones: Default::default(),
            material: self.material,
            springs: Vec::new(),
        }
    }

    /// Refuses defaults no avatar could be written with: text past the
    /// meta's limit.
    ///
    /// # Errors
    ///
    /// The first problem, as a sentence for the person.
    pub fn validate(&self) -> CoreResult<()> {
        self.spec("Avatar").meta.check_draft()
    }
}
