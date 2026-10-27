use serde_json::json;

use super::*;

#[test]
fn no_key_keeps_the_updater_off_whatever_the_build() {
    for debug in [false, true] {
        for env in [false, true] {
            assert_eq!(mode("", debug, env), UpdaterMode::Unreleased);
            assert_eq!(mode("   ", debug, env), UpdaterMode::Unreleased);
        }
    }
}

#[test]
fn a_debug_build_or_the_variable_turns_a_keyed_updater_off() {
    assert_eq!(mode("KEY", true, false), UpdaterMode::DevBuild);
    assert_eq!(mode("KEY", true, true), UpdaterMode::DevBuild);
    assert_eq!(mode("KEY", false, true), UpdaterMode::TurnedOff);
    assert_eq!(mode("KEY", false, false), UpdaterMode::On);
    assert!(mode("KEY", false, false).is_on());
    assert!(!mode("", false, false).is_on());
}

#[test]
fn the_variable_reads_like_a_switch() {
    for on in ["1", "true", "yes", " 1 "] {
        assert!(turned_off_by(Some(on)), "{on}");
    }
    for off in ["", " ", "0", "false", "FALSE"] {
        assert!(!turned_off_by(Some(off)), "{off}");
    }
    assert!(!turned_off_by(None));
}

#[test]
fn the_shipped_config_has_the_feed_and_no_key_yet() {
    let conf: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri.conf.json");
    let feed = FeedConfig::from_plugin_config(conf.pointer("/plugins/updater"));
    assert_eq!(
        feed.endpoint.as_deref(),
        Some("https://github.com/Shironex/maquette/releases/latest/download/latest.json")
    );
    // Wiring updates on is setting this one value; until then the app is inert.
    assert_eq!(mode(&feed.pubkey, false, false), UpdaterMode::Unreleased);
}

#[test]
fn a_missing_or_partial_section_reads_as_empty() {
    assert_eq!(FeedConfig::from_plugin_config(None), FeedConfig::default());
    let partial = json!({ "endpoints": [] });
    assert_eq!(
        FeedConfig::from_plugin_config(Some(&partial)),
        FeedConfig::default()
    );
    let keyed = json!({ "pubkey": "abc", "endpoints": ["https://a/latest.json", "https://b"] });
    let feed = FeedConfig::from_plugin_config(Some(&keyed));
    assert_eq!(feed.pubkey, "abc");
    assert_eq!(feed.endpoint.as_deref(), Some("https://a/latest.json"));
}

#[test]
fn no_release_yet_is_pending_not_an_error() {
    for pending in [
        // The feed answered 404 (private repo, nothing published).
        "Could not fetch a valid release JSON from the remote",
        // A release is out, but this platform's assets are still uploading.
        "the platform `darwin-aarch64` was not found in the response `platforms` object",
        "None of the fallback platforms `[\"windows-x86_64-nsis\"]` were found in the response `platforms` object",
        "HTTP status client error (404 Not Found) for url (https://github.com/Shironex/maquette/releases/latest/download/latest.json)",
    ] {
        assert!(is_release_pending(pending), "{pending}");
        assert_eq!(UpdaterEvent::failed(pending), UpdaterEvent::Pending);
    }
}

#[test]
fn real_failures_stay_failures() {
    for failure in [
        "error sending request for url (https://github.com/...): dns error",
        "The signature could not be verified",
        "HTTP status server error (500 Internal Server Error) for url (https://x/latest.json)",
        "404 page for something else",
    ] {
        assert!(!is_release_pending(failure), "{failure}");
        assert!(matches!(
            UpdaterEvent::failed(failure),
            UpdaterEvent::Failed { .. }
        ));
    }
}

#[test]
fn an_inert_updater_answers_off_and_offers_nothing() {
    let updates = Updates::new(
        UpdaterMode::Unreleased,
        Some("https://x/latest.json".into()),
    );
    let status = updates.status();
    assert_eq!(status.mode, UpdaterMode::Unreleased);
    assert_eq!(status.available, None);
    assert!(!status.ready);
}

#[test]
fn events_go_out_tagged() {
    let event = UpdaterEvent::Available {
        update: UpdateInfo {
            version: "0.2.0".into(),
            notes: Some("Faster bakes.".into()),
            date: Some("2026-11-02".into()),
        },
    };
    assert_eq!(
        serde_json::to_value(&event).expect("serialize"),
        json!({
            "type": "available",
            "update": { "version": "0.2.0", "notes": "Faster bakes.", "date": "2026-11-02" }
        })
    );
    assert_eq!(
        serde_json::to_value(UpdaterEvent::Progress {
            downloaded: 10.0,
            total: None
        })
        .expect("serialize"),
        json!({ "type": "progress", "downloaded": 10.0, "total": null })
    );
    assert_eq!(
        serde_json::to_value(UpdaterMode::DevBuild).expect("serialize"),
        json!("dev-build")
    );
}
