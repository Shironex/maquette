//! The standard views and the model facts over MCP: an agent reads what a
//! model measures and where it stands, hears what stops a render, and, with
//! a real Blender, gets the pictures beside the ref.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maquette_core::concepts::Concepts;
use maquette_core::inbox::fixture;
use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::pipeline::tools::{ResolvedTool, Toolbox};
use maquette_core::project::{NewProject, ProjectTemplate};
use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
use maquette_core::refs::{AssetRef, RefView};
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::{HostOs, ToolId};
use maquette_core::views::Views;
use maquette_core::{
    AppState, AssetKind, NewAsset, Stage, WorkspaceConfig, assets, project, workspace,
};
use maquette_desktop_lib::mcp::ensure_token;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::mcp::hub::Hub;
use maquette_desktop_lib::mcp::server::Running;
use maquette_desktop_lib::session::Session;
use rmcp::ServiceExt as _;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use super::support::{Client, call, call_error, raw_call, text};

/// A host with the views service and whatever Blender it is given.
struct ViewHost {
    session: Session,
    runner: PipelineRunner,
    concepts: Concepts,
    thumbs: Thumbnails,
    studio: ProviderJobs,
    views: Views,
    tools: Toolbox,
}

impl Host for ViewHost {
    fn session(&self) -> &Session {
        &self.session
    }
    fn runner(&self) -> &PipelineRunner {
        &self.runner
    }
    fn concepts(&self) -> &Concepts {
        &self.concepts
    }
    fn thumbs(&self) -> &Thumbnails {
        &self.thumbs
    }
    fn studio(&self) -> &ProviderJobs {
        &self.studio
    }
    fn toolbox(&self) -> Toolbox {
        self.tools.clone()
    }
    fn views(&self) -> Option<&Views> {
        Some(&self.views)
    }
}

/// The Blender to test with, as the core's Blender tests find it.
fn blender() -> Option<PathBuf> {
    let found = std::env::var_os("MAQUETTE_TEST_BLENDER")
        .map(PathBuf::from)
        .or_else(|| {
            [
                "/Applications/Blender.app/Contents/MacOS/Blender",
                "/usr/bin/blender",
                "C:\\Program Files\\Blender Foundation\\Blender\\blender.exe",
            ]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
        });
    if found.is_none() {
        eprintln!("no Blender found, skipped (set MAQUETTE_TEST_BLENDER to run it)");
    }
    found
}

fn toolbox(blender: &Path) -> Toolbox {
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program: blender.to_path_buf(),
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    tools
}

/// A workspace whose Moon Island has a book stack in two versions: v1 a
/// crate as a provider sent it, v2 the signpost a script rebuilt it as, with
/// a front ref, a real size, an origin fix-up and a place in the layout.
fn workspace_with_a_rebuild(dir: &Path) -> (Session, PathBuf) {
    let root = dir.join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let session = Session::with_state(dir.join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let moon = project::dir(&root, &config.layout, "moon-island").expect("dir");
    for folder in ["models", "refs"] {
        std::fs::create_dir_all(moon.join(folder)).expect("folder");
    }
    std::fs::write(moon.join("models/book_stack_v1.glb"), fixture::clean_prop()).expect("v1");
    std::fs::write(moon.join("models/book_stack_v2.glb"), fixture::signpost()).expect("v2");
    std::fs::write(moon.join("refs/book-stack.png"), fixture::ref_card()).expect("ref");
    project::modify(&root, &config.layout, "moon-island", |m| {
        let stack = assets::add(
            m,
            NewAsset {
                name: "Book stack".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )?;
        for file in ["models/book_stack_v1.glb", "models/book_stack_v2.glb"] {
            assets::add_version(m, &stack.id, file.into(), None)?;
        }
        Ok(())
    })
    .expect("asset");
    maquette_core::refs::set_views(
        &root,
        &config.layout,
        "moon-island",
        "book-stack",
        &[AssetRef {
            file: "refs/book-stack.png".into(),
            view: RefView::Front,
        }],
    )
    .expect("ref view");
    let write = |name: &str, value: Value| {
        std::fs::write(
            moon.join(name),
            serde_json::to_vec_pretty(&value).expect("json"),
        )
        .expect("write");
    };
    write(
        "prepare.json",
        json!({
            "version": 1,
            "models": { "book-stack": {
                "size": { "axis": "height", "value": 18, "unit": "cm" },
                "fixups": [{ "op": "origin-bottom" }],
            } },
        }),
    );
    write(
        "layout.json",
        json!({
            "version": 1,
            "models": { "book-stack": { "position": [-0.68, 0.0, -0.27], "rotation": [0, 18, 0], "scale": 0.44 } },
            "stops": [],
        }),
    );
    (session, moon)
}

async fn connect(session: Session, tools: Toolbox, views: Views) -> (Client, Running) {
    let host = Arc::new(ViewHost {
        session,
        runner: PipelineRunner::new(Arc::new(NullSink)),
        concepts: Concepts::new(None, 1, |_| {}),
        thumbs: Thumbnails::new(|_| {}),
        studio: ProviderJobs::new(
            Arc::new(MemoryStore::default()),
            Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        ),
        views,
        tools,
    });
    let token = ensure_token(&host.session).expect("token");
    let server = maquette_desktop_lib::mcp::server::start(
        0,
        token.clone(),
        host.clone() as Arc<dyn Host>,
        Arc::new(Hub::new(|| {})),
    )
    .expect("start");
    let url = format!("http://127.0.0.1:{}/mcp", server.port());
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url).auth_header(token),
    );
    let client = ClientConfig::default()
        .serve(transport)
        .await
        .expect("initialize");
    (client, server)
}

/// `base` and the folders of `path`, joined one by one, as the app builds
/// its paths: a string compare then holds on Windows too.
fn under(base: &Path, path: &str) -> PathBuf {
    path.split('/')
        .fold(base.to_path_buf(), |at, part| at.join(part))
}

fn close(value: &Value, want: [f64; 3]) -> bool {
    (0..3).all(|k| (value[k].as_f64().unwrap_or(f64::NAN) - want[k]).abs() < 2e-3)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_read_a_models_facts_without_a_script_of_their_own() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, moon) = workspace_with_a_rebuild(dir.path());
    let (client, server) = connect(session, Toolbox::default(), Views::default()).await;

    // By name, the newest version: the rebuilt one.
    let facts = call(
        &client,
        "get_model_facts",
        json!({ "project": "moon-island", "asset": "Book stack" }),
    )
    .await;
    assert_eq!(facts["asset"], "book-stack");
    assert_eq!(
        (facts["version"].clone(), facts["newest"].clone()),
        (json!(2), json!(2))
    );
    assert_eq!(facts["counts"]["triangles"], 48);
    assert_eq!(facts["counts"]["meshes"], 4);
    assert_eq!(facts["counts"]["objects"], 4);
    assert!(
        close(&facts["inFile"]["size"], [0.46, 0.36, 0.26]),
        "{}",
        facts["inFile"]
    );
    // 36 cm in the file, 18 cm real: half size, the bottom centre at the origin.
    assert_eq!(facts["realSize"]["text"], "18 cm high");
    assert_eq!(facts["realSize"]["scale"], json!(0.5));
    assert!(
        close(&facts["atRealSize"]["size"], [0.23, 0.18, 0.13]),
        "{}",
        facts["atRealSize"]
    );
    assert!(close(&facts["atRealSize"]["min"], [-0.115, 0.0, -0.065]));
    // The layout entry as layout.json has it.
    assert_eq!(facts["layout"]["placed"], true);
    assert_eq!(
        facts["layout"]["entry"]["position"],
        json!([-0.68, 0.0, -0.27])
    );
    assert_eq!(
        facts["layout"]["entry"]["rotation"],
        json!([0.0, 18.0, 0.0])
    );
    assert_eq!(facts["layout"]["entry"]["scale"], json!(0.44));
    assert!(facts["layout"]["inScene"]["size"].is_array());
    // A prop's front is the format's, and the answer says nobody measured it.
    assert_eq!(facts["facing"]["inFile"]["axis"], "+z");
    assert_eq!(facts["facing"]["inFile"]["blender"], "-y");
    assert_eq!(facts["facing"]["how"], "convention");
    assert_eq!(facts["facing"]["sure"], "assumed");
    assert!(
        facts["facing"]["note"]
            .as_str()
            .expect("note")
            .starts_with("Not measured")
    );
    assert_eq!(facts["facing"]["inScene"]["degrees"], json!(18.0));
    // The ref comes with its whole path, so nothing has to be looked up.
    assert_eq!(facts["refs"][0]["view"], "front");
    assert_eq!(
        facts["refs"][0]["file"],
        json!(moon.join("refs/book-stack.png"))
    );
    assert_eq!(facts["file"], json!(moon.join("models/book_stack_v2.glb")));
    assert!(
        facts["axes"]
            .as_str()
            .expect("axes")
            .contains("front is +z")
    );
    assert_eq!(facts["next"][0]["tool"], "render_views");

    // The version it replaces: one mesh, another size in the file, and said so.
    let old = call(
        &client,
        "get_model_facts",
        json!({ "project": "moon-island", "asset": "book-stack", "version": 1 }),
    )
    .await;
    assert_eq!(old["counts"]["objects"], 1);
    assert_eq!(old["counts"]["triangles"], 12);
    assert!(close(&old["inFile"]["size"], [0.5, 0.5, 0.5]));
    // The same real size by another scale: both versions fill the same height.
    assert_eq!(old["realSize"]["scale"], json!(0.36));
    assert!(close(&old["atRealSize"]["size"], [0.18, 0.18, 0.18]));
    let notes = old["notes"].to_string();
    assert!(
        notes.contains("one mesh") && notes.contains("not the newest (v2)"),
        "{notes}"
    );

    let unknown = call_error(
        &client,
        "get_model_facts",
        json!({ "project": "moon-island", "asset": "kettle" }),
    )
    .await;
    assert!(
        unknown.contains("Unknown asset") && unknown.contains("book-stack"),
        "{unknown}"
    );
    let version = call_error(
        &client,
        "get_model_facts",
        json!({ "project": "moon-island", "asset": "book-stack", "version": 9 }),
    )
    .await;
    assert!(
        version.contains("has no v9. Its versions: v1, v2."),
        "{version}"
    );

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn render_views_checks_what_it_is_asked_and_says_what_stops_it() {
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, _) = workspace_with_a_rebuild(dir.path());
    let (client, server) = connect(session, Toolbox::default(), Views::default()).await;
    let asked = |more: Value| {
        let mut args = json!({ "project": "moon-island", "asset": "book-stack" });
        args.as_object_mut()
            .expect("object")
            .extend(more.as_object().expect("more").clone());
        args
    };

    // Without Blender nothing renders, and the answer says who can fix that.
    let none = raw_call(&client, "render_views", asked(json!({ "asIs": true }))).await;
    assert_eq!(none.is_error, Some(true));
    let body: Value = serde_json::from_str(&text(&none)).expect("json");
    assert!(
        body["error"]
            .as_str()
            .expect("error")
            .contains("Blender was not found"),
        "{body}"
    );
    assert_eq!(body["personOnly"], true);
    assert_eq!(body["tryTool"], "get_started");
    // The asset's real size has to be applied first, which is Blender's work too.
    let prepared = call_error(&client, "render_views", asked(json!({}))).await;
    assert!(
        prepared.contains("Blender is not installed, so the fix-ups cannot run"),
        "{prepared}"
    );

    for (args, said) in [
        (
            json!({ "angle": { "azimuth": 400 } }),
            "azimuth from -180 to 180",
        ),
        (
            json!({ "angle": { "azimuth": 30, "elevation": 85 } }),
            "elevation from 0 to 80",
        ),
        (
            json!({ "closeUps": [{ "half": 0 }, { "half": 1 }, { "half": 0 }] }),
            "2 close ups",
        ),
        (
            json!({ "closeUps": [{ "half": 0, "node": "body" }] }),
            "exactly one of node, box and half",
        ),
        (
            json!({ "closeUps": [{}] }),
            "exactly one of node, box and half",
        ),
        (
            json!({ "closeUps": [{ "box": [[0, 0, 0], [0, 1, 1]] }] }),
            "shares of the bounding box",
        ),
        (
            json!({ "compare": 2 }),
            "another version than the one rendered",
        ),
        (json!({ "compare": 7 }), "has no v7. Its versions: v1, v2."),
        (json!({ "version": 5 }), "has no v5"),
    ] {
        let err = call_error(&client, "render_views", asked(args)).await;
        assert!(err.contains(said), "{said}: {err}");
    }
    let unknown = call_error(
        &client,
        "render_views",
        json!({ "project": "moon-island", "asset": "kettle" }),
    )
    .await;
    assert!(unknown.contains("Unknown asset"), "{unknown}");

    // A cancel with nothing rendering says so, and names the folder it means.
    let idle = call(
        &client,
        "render_views",
        asked(json!({ "cancel": true, "compare": 1 })),
    )
    .await;
    assert_eq!(idle["status"], "nothing was rendering");
    let folder = idle["dir"].as_str().expect("dir").replace('\\', "/");
    assert!(
        folder.ends_with("moon-island/exports/renders/book-stack/views/v2-vs-v1"),
        "{folder}"
    );

    client.cancel().await.expect("close");
    server.stop();
}

/// Calls render_views until the set is there: a busy machine can outlast
/// one wait.
async fn rendered(client: &Client, args: Value) -> Value {
    for _ in 0..10 {
        let answer = call(client, "render_views", args.clone()).await;
        if answer["status"] == "ready" {
            return answer;
        }
        assert_eq!(answer["status"], "rendering", "{answer}");
        assert!(
            answer["note"]
                .as_str()
                .expect("note")
                .contains("Call render_views again")
        );
    }
    panic!("the views never finished");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_gets_a_rebuild_beside_its_ref_and_beside_what_it_replaces() {
    let Some(blender) = blender() else {
        return;
    };
    maquette_core::providers::http::ensure_crypto();
    let dir = tempfile::tempdir().expect("temp dir");
    let (session, moon) = workspace_with_a_rebuild(dir.path());
    // Small pictures keep the test quick; the app's are 1024 px.
    let (client, server) = connect(session, toolbox(&blender), Views::sized(192)).await;
    let args = json!({ "project": "moon-island", "asset": "book-stack", "waitSeconds": 240 });

    let set = rendered(&client, args.clone()).await;
    assert_eq!(
        (set["version"].clone(), set["size"].clone()),
        (json!(2), json!(192))
    );
    let folder = under(&moon, "exports/renders/book-stack/views/v2");
    assert_eq!(set["dir"], json!(folder));
    let views = set["views"].as_array().expect("views");
    let ids: Vec<&str> = views
        .iter()
        .map(|v| v["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        ["three-quarter", "front", "top", "close-up-1", "close-up-2"]
    );
    for view in views {
        let file = Path::new(view["file"].as_str().expect("file"));
        assert!(
            file.starts_with(&folder) && file.is_file(),
            "{}",
            file.display()
        );
        assert!(
            view["camera"].as_str().expect("camera").contains("degrees") || view["id"] == "top"
        );
        assert!(view["frames"].is_string());
    }
    assert_eq!(
        views[1]["camera"],
        "straight in front, on the model's +Z side, 6 degrees up"
    );
    assert!(Path::new(set["sheet"]["file"].as_str().expect("sheet")).is_file());
    assert!(
        set["sheet"]["shows"]
            .as_str()
            .expect("shows")
            .contains("the asset's refs first")
    );
    // The ref's path comes with the pictures.
    assert_eq!(
        set["refs"][0]["file"],
        json!(moon.join("refs/book-stack.png"))
    );
    assert_eq!(set["refs"][0]["view"], "front");
    // The fix-ups and the real size were applied: 18 cm high, standing on the floor.
    assert!(
        set["prepared"]
            .as_str()
            .expect("prepared")
            .contains("real size are applied")
    );
    let bounds = &set["boundsRendered"][0];
    assert!(close(&bounds["min"], [-0.115, 0.0, -0.065]), "{bounds}");
    assert!(close(&bounds["max"], [0.115, 0.18, 0.065]), "{bounds}");
    assert_eq!(set["lightNumbers"]["exposure"], json!(1.15));
    assert_eq!(set["next"][0]["tool"], "get_model_facts");

    // Asked again, the set answers from its folder.
    let again = call(&client, "render_views", args).await;
    assert_eq!(again["status"], "ready");
    assert_eq!(again["views"], set["views"]);

    // Side by side with the version it replaces: the same cameras, both at real size.
    let both = rendered(
        &client,
        json!({ "project": "moon-island", "asset": "book-stack", "compare": 1, "waitSeconds": 240 }),
    )
    .await;
    assert_eq!(both["compare"], 1);
    let pair = under(&moon, "exports/renders/book-stack/views/v2-vs-v1");
    let (new, old) = (
        both["views"].as_array().expect("views"),
        both["compared"]["views"].as_array().expect("compared"),
    );
    assert_eq!(both["compared"]["version"], 1);
    for (a, b) in new.iter().zip(old) {
        assert_eq!(
            (&a["position"], &a["target"]),
            (&b["position"], &b["target"])
        );
        assert_eq!(
            a["file"],
            json!(pair.join(format!("v2_{}.png", a["id"].as_str().expect("id"))))
        );
        assert_eq!(
            b["file"],
            json!(pair.join(format!("v1_{}.png", b["id"].as_str().expect("id"))))
        );
        assert!(Path::new(b["file"].as_str().expect("file")).is_file());
    }
    // Both stand 18 cm high: the real size holds for whichever version is asked.
    for row in both["boundsRendered"].as_array().expect("rows") {
        let top = row["max"][1].as_f64().expect("top");
        assert!((top - 0.18).abs() < 2e-3, "{row}");
    }
    assert!(
        both["sheet"]["shows"]
            .as_str()
            .expect("shows")
            .contains("a row per version")
    );

    client.cancel().await.expect("close");
    server.stop();
}
