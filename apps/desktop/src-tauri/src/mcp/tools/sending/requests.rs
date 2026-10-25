//! Building the send requests from tool arguments.

use maquette_core::providers::{
    BatchItem, BatchRequest, MeshyPreset, PresetVariant, Provider, ProviderId, SendRequest,
    TripoPreset,
};
use serde::Serialize;
use serde_json::Value;

use super::{ProviderArg, SendBatchArgs, SendTo3dArgs, ServiceArg, VariantArg};
use crate::commands::providers;
use crate::mcp::host::Host;
use crate::mcp::tools::{to_value, workspace};

/// Lays `overrides` over a preset, refusing names the preset does not have.
fn with_overrides<T: Serialize + serde::de::DeserializeOwned>(
    base: &T,
    overrides: Option<&serde_json::Map<String, Value>>,
) -> Result<T, String> {
    let Some(overrides) = overrides.filter(|o| !o.is_empty()) else {
        return serde_json::from_value(to_value(base)).map_err(|e| e.to_string());
    };
    let mut value = to_value(base);
    let Some(fields) = value.as_object_mut() else {
        return Err("The preset could not be read.".to_string());
    };
    for (key, field) in overrides {
        if !fields.contains_key(key) {
            let mut known: Vec<&str> = fields.keys().map(String::as_str).collect();
            known.sort_unstable();
            return Err(format!(
                "There is no preset setting {key:?}. The settings are {}.",
                known.join(", ")
            ));
        }
        fields.insert(key.clone(), field.clone());
    }
    serde_json::from_value(value).map_err(|e| format!("The overrides do not fit the preset. {e}"))
}

/// Builds the send an agent asked for, filling in what it left out.
pub(super) fn send_request(host: &dyn Host, args: &SendTo3dArgs) -> Result<SendRequest, String> {
    let (_, config) = workspace(host)?;
    let options =
        providers::send_options(host.session(), host.studio(), &args.project, &args.asset)
            .map_err(|e| e.message)?;
    let variant = match args.variant {
        Some(VariantArg::Prop) => PresetVariant::Prop,
        Some(VariantArg::Character) => PresetVariant::Character,
        None => options.variant,
    };
    let images = if args.images.is_empty() {
        options.suggested.clone()
    } else {
        args.images.clone()
    };
    if images.is_empty() {
        return Err(format!(
            "{} has no refs of its own yet. Pass images, for example [\"refs/{}_front.png\"].",
            options.asset.name,
            options.asset.id.replace('-', "_")
        ));
    }
    let (provider, service) = match args.provider {
        ProviderArg::Tripo => (Provider::Tripo, ProviderId::Tripo),
        ProviderArg::Meshy => (Provider::Meshy, ProviderId::Meshy),
        ProviderArg::ByHand => (
            Provider::ByHand,
            match args.service {
                Some(ServiceArg::Meshy) => ProviderId::Meshy,
                _ => ProviderId::Tripo,
            },
        ),
    };
    let overrides = args.overrides.as_ref();
    let (tripo, meshy): (Option<TripoPreset>, Option<MeshyPreset>) = match service {
        ProviderId::Tripo => (
            Some(with_overrides(config.providers.tripo(variant), overrides)?),
            None,
        ),
        ProviderId::Meshy => (
            None,
            Some(with_overrides(config.providers.meshy(variant), overrides)?),
        ),
    };
    Ok(SendRequest {
        project: args.project.clone(),
        asset: options.asset.id,
        provider,
        service,
        variant,
        images,
        tripo,
        meshy,
        prompt: None,
        redo_of: None,
    })
}

fn variant(arg: Option<VariantArg>, fallback: PresetVariant) -> PresetVariant {
    match arg {
        Some(VariantArg::Prop) => PresetVariant::Prop,
        Some(VariantArg::Character) => PresetVariant::Character,
        None => fallback,
    }
}

/// Builds the batch an agent asked for: each asset's own refs and kind
/// unless given, one preset per variant with the overrides laid over both,
/// and an asset's own overrides laid over its variant's preset.
pub(super) fn batch_request(host: &dyn Host, args: &SendBatchArgs) -> Result<BatchRequest, String> {
    let (_, config) = workspace(host)?;
    let ids: Vec<String> = args.items.iter().map(|i| i.asset.clone()).collect();
    let options = providers::batch_options(host.session(), host.studio(), &args.project, &ids)
        .map_err(|e| e.message)?;
    let (provider, service) = match args.provider {
        ProviderArg::Tripo => (Provider::Tripo, ProviderId::Tripo),
        ProviderArg::Meshy => (Provider::Meshy, ProviderId::Meshy),
        ProviderArg::ByHand => (
            Provider::ByHand,
            match args.service {
                Some(ServiceArg::Meshy) => ProviderId::Meshy,
                _ => ProviderId::Tripo,
            },
        ),
    };
    let overrides = args.overrides.as_ref();
    let p = &config.providers;
    let tripo = service == ProviderId::Tripo;
    let tripo_for = |variant| with_overrides(p.tripo(variant), overrides);
    let meshy_for = |variant| with_overrides(p.meshy(variant), overrides);
    let mut items = Vec::new();
    for arg in &args.items {
        let Some(found) = options.items.iter().find(|o| o.asset.id == arg.asset) else {
            return Err(format!("{} is in the batch twice.", arg.asset));
        };
        let images = if arg.images.is_empty() {
            found.suggested.clone()
        } else {
            arg.images.clone()
        };
        if images.is_empty() {
            return Err(format!(
                "{} has no refs of its own yet. Pass its images, for example [\"refs/{}.png\"].",
                found.asset.name,
                found.asset.id.replace('-', "_")
            ));
        }
        let variant = variant(arg.variant, found.variant);
        let own = arg.overrides.as_ref().filter(|o| !o.is_empty());
        let name = &found.asset.name;
        let tag = |err: String| format!("{name}: {err}");
        items.push(BatchItem {
            asset: found.asset.id.clone(),
            variant,
            images,
            tripo: own
                .filter(|_| tripo)
                .map(|o| with_overrides(&tripo_for(variant)?, Some(o)))
                .transpose()
                .map_err(tag)?,
            meshy: own
                .filter(|_| !tripo)
                .map(|o| with_overrides(&meshy_for(variant)?, Some(o)))
                .transpose()
                .map_err(tag)?,
            prompt: None,
            redo_of: None,
        });
    }
    Ok(BatchRequest {
        project: args.project.clone(),
        provider,
        service,
        items,
        tripo_prop: tripo.then(|| tripo_for(PresetVariant::Prop)).transpose()?,
        tripo_character: tripo
            .then(|| tripo_for(PresetVariant::Character))
            .transpose()?,
        meshy_prop: (!tripo)
            .then(|| meshy_for(PresetVariant::Prop))
            .transpose()?,
        meshy_character: (!tripo)
            .then(|| meshy_for(PresetVariant::Character))
            .transpose()?,
    })
}
