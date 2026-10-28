//! The Image API client against a local fake server: nothing reaches
//! OpenAI and no image is paid for.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use base64::Engine as _;
use serde_json::json;

use super::*;
use crate::providers::fake::{FakeServer, Reply};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn image_answer(usage: Option<serde_json::Value>) -> serde_json::Value {
    let mut answer = json!({
        "created": 1_761_600_000,
        "data": [{ "b64_json": b64(PNG) }],
        "output_format": "png",
        "quality": "low",
        "size": "1024x1024",
    });
    if let Some(usage) = usage {
        answer["usage"] = usage;
    }
    answer
}

fn call(images: Vec<PathBuf>) -> ImageCall {
    ImageCall {
        prompt: "Generate an image.\n\nA lantern.".into(),
        images,
        size: "1024x1024".into(),
        quality: ImageQuality::Low,
        transparent: false,
    }
}

fn api(server: &FakeServer) -> Api {
    Api::new(SERVICE, &server.url, "sk-test-0000", Timing::instant()).expect("api")
}

#[test]
fn a_prompt_alone_is_a_generation_and_its_usage_is_priced() {
    let server = FakeServer::start(|_, req| {
        assert_eq!(req.path, "/images/generations");
        Reply::ok(&image_answer(Some(json!({
            "input_tokens": 1000,
            "output_tokens": 2000,
            "total_tokens": 3000,
            "input_tokens_details": { "text_tokens": 1000, "image_tokens": 0 }
        }))))
    });
    let made = make(&api(&server), &StopFlag::default(), &call(Vec::new())).expect("made");
    assert_eq!(made.bytes, PNG);
    assert!(!made.estimated);
    // 1000 text in at $5/M and 2000 out at $30/M.
    assert!((made.usd - 0.065).abs() < 1e-9, "{}", made.usd);
    let sent = server
        .find("POST", "/images/generations")
        .expect("sent")
        .json();
    assert_eq!(sent["model"], MODEL);
    assert_eq!(sent["quality"], "low");
    assert_eq!(sent["output_format"], "png");
    assert_eq!(sent["n"], 1);
    assert!(sent.get("images").is_none());
    assert!(sent.get("background").is_none());
    let auth = server.requests()[0]
        .header("authorization")
        .map(str::to_string);
    assert_eq!(auth.as_deref(), Some("Bearer sk-test-0000"));
}

#[test]
fn attached_images_make_an_edit_in_their_order() {
    let dir = tempfile::tempdir().expect("dir");
    let first = dir.path().join("front.png");
    let second = dir.path().join("board.jpg");
    std::fs::write(&first, PNG).expect("write");
    std::fs::write(&second, b"jpeg bytes").expect("write");
    let server = FakeServer::start(|_, _| Reply::ok(&image_answer(None)));
    let mut transparent = call(vec![first, second]);
    transparent.transparent = true;
    transparent.size = "auto".into();
    let made = make(&api(&server), &StopFlag::default(), &transparent).expect("made");
    assert!(made.estimated, "no usage in the answer");
    assert!((made.usd - estimate_usd(ImageQuality::Low, 2)).abs() < 1e-12);
    let sent = server.find("POST", "/images/edits").expect("sent").json();
    let images = sent["images"].as_array().expect("images");
    assert_eq!(images.len(), 2);
    assert_eq!(
        images[0]["image_url"],
        format!("data:image/png;base64,{}", b64(PNG))
    );
    assert!(
        images[1]["image_url"]
            .as_str()
            .expect("url")
            .starts_with("data:image/jpeg;base64,")
    );
    assert_eq!(sent["background"], "transparent");
    assert_eq!(sent["size"], "auto");
}

#[test]
fn a_429_waits_for_retry_after_then_goes_on() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let server = FakeServer::start(move |_, _| {
        if counter.fetch_add(1, Ordering::SeqCst) == 0 {
            Reply::json(
                429,
                &json!({ "error": { "message": "Rate limit reached" } }),
            )
            .with_header("Retry-After", "0")
        } else {
            Reply::ok(&image_answer(None))
        }
    });
    let made = make(&api(&server), &StopFlag::default(), &call(Vec::new())).expect("made");
    assert_eq!(made.bytes, PNG);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[test]
fn errors_read_openais_message_and_point_at_the_settings() {
    let server = FakeServer::start(|_, req| {
        if req.path.starts_with("/images") {
            Reply::json(
                400,
                &json!({ "error": { "message": "Your prompt was rejected.", "type": "invalid_request_error" } }),
            )
        } else {
            Reply::json(401, &json!({ "error": { "message": "Incorrect API key" } }))
        }
    });
    let err = make(&api(&server), &StopFlag::default(), &call(Vec::new())).expect_err("400");
    assert_eq!(
        error_message(&err),
        "OpenAI answered 400: Your prompt was rejected."
    );
    let tested = test_key(&server.url, "sk-bad", Timing::instant());
    assert!(!tested.ok);
    assert!(tested.message.contains("Settings > Image generation"));
    assert!(!tested.message.contains("sk-bad"));
}

#[test]
fn test_key_only_lists_the_models() {
    let server = FakeServer::start(|_, _| {
        Reply::ok(&json!({ "object": "list", "data": [
            { "id": "gpt-image-2", "object": "model" },
            { "id": MODEL, "object": "model" }
        ] }))
    });
    let tested = test_key(&server.url, "sk-good", Timing::instant());
    assert!(tested.ok && tested.has_model, "{tested:?}");
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/models");

    let without = FakeServer::start(|_, _| Reply::ok(&json!({ "data": [{ "id": "gpt-6-luna" }] })));
    let tested = test_key(&without.url, "sk-good", Timing::instant());
    assert!(tested.ok && !tested.has_model);
    assert!(tested.message.contains("does not list"));
}

#[test]
fn inputs_that_cannot_go_are_refused_before_sending() {
    let dir = tempfile::tempdir().expect("dir");
    let gif = dir.path().join("old.gif");
    std::fs::write(&gif, b"GIF89a").expect("write");
    let server = FakeServer::start(|_, _| Reply::ok(&image_answer(None)));
    let err = make(&api(&server), &StopFlag::default(), &call(vec![gif])).expect_err("gif");
    assert!(matches!(err, ApiError::Malformed(ref m) if m.contains("old.gif")));
    let many = (0..=MAX_IMAGES)
        .map(|n| dir.path().join(format!("{n}.png")))
        .collect();
    assert!(make(&api(&server), &StopFlag::default(), &call(many)).is_err());
    assert!(server.requests().is_empty(), "nothing was sent");
}

#[test]
fn the_estimate_grows_with_quality_and_images() {
    let low = estimate_usd(ImageQuality::Low, 0);
    let high = estimate_usd(ImageQuality::High, 0);
    assert!(low > 0.0 && low < 0.02, "{low}");
    assert!(high > 0.2, "{high}");
    assert!(estimate_usd(ImageQuality::Low, 3) > low);
    let usage = Usage {
        input_tokens: 500,
        output_tokens: 0,
        input_tokens_details: None,
    };
    assert!(
        (cost_usd(&usage) - 0.004).abs() < 1e-12,
        "image price for unsplit input"
    );
}
