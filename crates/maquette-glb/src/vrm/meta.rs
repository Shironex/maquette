//! The VRM 1.0 meta: who made the avatar and what others may do with it.
//! Field names and defaults follow the spec: only the name, at least one
//! author and the licence URL are required, and every permission defaults
//! to the strictest choice.

use serde::{Deserialize, Serialize};

use maquette_base::error::{CoreError, CoreResult};

/// The VRM 1.0 licence every VRM file points to.
pub const LICENSE_URL: &str = "https://vrm.dev/licenses/1.0/";

/// Who may play the avatar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AvatarPermission {
    #[default]
    OnlyAuthor,
    OnlySeparatelyLicensedPerson,
    Everyone,
}

/// Who may use the avatar for money.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CommercialUsage {
    #[default]
    PersonalNonProfit,
    PersonalProfit,
    Corporation,
}

/// Whether a user must credit the author.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CreditNotation {
    #[default]
    Required,
    Unnecessary,
}

/// Whether others may change the avatar, and hand the change on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Modification {
    #[default]
    Prohibited,
    AllowModification,
    AllowModificationRedistribution,
}

/// The `meta` of `VRMC_vrm`, as written (its thumbnail index aside).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct VrmMeta {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright_information: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_information: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub third_party_licenses: Option<String>,
    pub license_url: String,
    pub avatar_permission: AvatarPermission,
    pub allow_excessively_violent_usage: bool,
    pub allow_excessively_sexual_usage: bool,
    pub commercial_usage: CommercialUsage,
    pub allow_political_or_religious_usage: bool,
    pub allow_antisocial_or_hate_usage: bool,
    pub credit_notation: CreditNotation,
    pub allow_redistribution: bool,
    pub modification: Modification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_license_url: Option<String>,
}

impl Default for VrmMeta {
    fn default() -> Self {
        VrmMeta {
            name: String::new(),
            version: None,
            authors: Vec::new(),
            copyright_information: None,
            contact_information: None,
            references: Vec::new(),
            third_party_licenses: None,
            license_url: LICENSE_URL.to_string(),
            avatar_permission: AvatarPermission::default(),
            allow_excessively_violent_usage: false,
            allow_excessively_sexual_usage: false,
            commercial_usage: CommercialUsage::default(),
            allow_political_or_religious_usage: false,
            allow_antisocial_or_hate_usage: false,
            credit_notation: CreditNotation::default(),
            allow_redistribution: false,
            modification: Modification::default(),
            other_license_url: None,
        }
    }
}

/// The longest text a meta field may hold.
pub const META_TEXT_MAX: usize = 2000;

impl VrmMeta {
    /// Refuses a meta VRM would refuse: no name, no author, or anything
    /// [`VrmMeta::check_draft`] refuses.
    ///
    /// # Errors
    ///
    /// The first problem, as a sentence for the person or agent.
    pub fn validate(&self) -> CoreResult<()> {
        if self.name.trim().is_empty() {
            return Err(invalid("The avatar needs a name."));
        }
        if self.authors.is_empty() {
            return Err(invalid("Name at least one author."));
        }
        self.check_draft()
    }

    /// Checks what a meta still being filled in may already hold: no blank
    /// author, VRM's licence URL, and text up to [`META_TEXT_MAX`]
    /// characters. The name and authors may still be missing.
    ///
    /// # Errors
    ///
    /// The first problem, as a sentence for the person or agent.
    pub fn check_draft(&self) -> CoreResult<()> {
        if self.authors.iter().any(|a| a.trim().is_empty()) {
            return Err(invalid("An author name is blank. Fill it in or remove it."));
        }
        if self.license_url != LICENSE_URL {
            return Err(invalid(format!(
                "The licence URL must be {LICENSE_URL}. Put another licence in the other licence URL."
            )));
        }
        let texts = [
            Some(&self.name),
            self.version.as_ref(),
            self.copyright_information.as_ref(),
            self.contact_information.as_ref(),
            self.third_party_licenses.as_ref(),
            self.other_license_url.as_ref(),
        ];
        let long = texts
            .into_iter()
            .flatten()
            .chain(&self.authors)
            .chain(&self.references)
            .any(|t| t.chars().count() > META_TEXT_MAX);
        if long {
            return Err(invalid(format!(
                "Keep each meta field to {META_TEXT_MAX} characters or fewer."
            )));
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}
