//! The rig doctor's tools: an agent can start and read it, and no tool
//! approves a joint move or lands a draft.

const NO_TOOL_MAY_DO: [&str; 4] = [
    "approve_rig_doctor",
    "land_rig_doctor",
    "decide_rig_doctor",
    "move_rig_joints",
];

#[test]
fn the_tools_can_start_and_read_the_doctor_but_never_approve() {
    let names = include_str!("../../tool-names.txt");
    for tool in [
        "start_rig_doctor",
        "run_rig_doctor",
        "get_rig_doctor",
        "list_rig_doctors",
    ] {
        assert!(names.lines().any(|l| l == tool), "{tool} is listed");
    }
    for tool in NO_TOOL_MAY_DO {
        assert!(!names.lines().any(|l| l == tool), "{tool} must not exist");
    }
}

mod rigid {
    //! An agent asks for a rigid bind through run_rig_doctor: it may ask,
    //! the doctor refuses a part or bone the model lacks, and nothing it
    //! asks for becomes the draft without a person.

    use std::sync::Arc;
    use std::time::Duration;

    use maquette_core::concepts::Concepts;
    use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
    use maquette_core::pipeline::tools::Toolbox;
    use maquette_core::providers::{MemoryStore, Options, ProviderJobs};
    use maquette_core::rig::{RigDoctors, RigRecord};
    use maquette_core::thumbs::Thumbnails;
    use maquette_core::{AppState, WorkspaceConfig, workspace};
    use maquette_desktop_lib::mcp::ensure_token;
    use maquette_desktop_lib::mcp::host::Host;
    use maquette_desktop_lib::mcp::hub::Hub;
    use maquette_desktop_lib::session::Session;
    use rmcp::ServiceExt as _;
    use rmcp::model::ClientConfig;
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
    use serde_json::json;

    use super::super::support::{call, call_error};

    /// Blender is never there: every script fails, as on a computer without it.
    struct NoBlender;

    impl maquette_core::base::ports::ScriptRunner for NoBlender {
        fn run_script(
            &self,
            _: &Toolbox,
            _: &std::path::Path,
            _: (&str, &str),
            _: &[String],
        ) -> maquette_core::CoreResult<()> {
            Err(maquette_core::CoreError::Invalid(
                "Blender is not installed.".into(),
            ))
        }
    }

    struct RigHost {
        session: Session,
        runner: PipelineRunner,
        concepts: Concepts,
        thumbs: Thumbnails,
        studio: ProviderJobs,
        rig: RigDoctors,
    }

    impl Host for RigHost {
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
            Toolbox::default()
        }
        fn rig(&self) -> Option<&RigDoctors> {
            Some(&self.rig)
        }
    }

    /// A humanoid case a person fixed, with a wing on each arm.
    fn fixed_case(dir: &std::path::Path) -> RigRecord {
        let model = json!({
            "rig": "Mixamo, 65 joints", "joints": 65, "skinnedMeshes": 1, "clips": [],
            "triangles": 1000, "height": 1.0, "textures": 1, "handFar": 0, "armFar": 900,
            "islands": 1, "oneIsland": true, "finding": null,
        });
        let part = |n: u32, side: &str| json!({ "part": n, "vertices": 900, "side": side, "bones": [], "acrossCm": 38.0 });
        serde_json::from_value(json!({
            "id": "moth-v1", "name": "Moth", "startedBy": "person", "route": "humanoid",
            "state": "fixed", "step": "land",
            "source": { "inbox": null, "project": "p", "asset": "moth", "version": 1 },
            "input": dir.join("in.glb"), "dir": dir, "model": model, "issues": [],
            "creature": null, "before": null,
            "after": null, "defects": [], "stats": [], "fixSteps": [], "checks": [],
            "jointMove": null, "round": 1, "draft": dir.join("draft.glb"), "progress": null,
            "stages": [], "log": [], "stop": null, "landed": null, "error": null,
            "updatedMs": 0.0, "parts": [part(1, "left"), part(2, "right")],
            "bodyBones": ["mixamorig:Spine2", "mixamorig:Hips"],
        }))
        .expect("record")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_agent_asks_for_a_bind_and_a_part_or_bone_the_model_lacks_is_refused() {
        maquette_core::providers::http::ensure_crypto();
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let work = maquette_core::rig::work_dir(&root, &config, "moth-v1");
        maquette_core::rig::save(&fixed_case(&work)).expect("save");
        let studio = ProviderJobs::new(
            Arc::new(MemoryStore::default()),
            Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        );
        let host = Arc::new(RigHost {
            session,
            runner: PipelineRunner::new(Arc::new(NullSink)),
            concepts: Concepts::new(None, 1, |_| {}),
            thumbs: Thumbnails::new(|_| {}),
            studio,
            rig: RigDoctors::new(Arc::new(NoBlender), |_| {}),
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

        let read = call(&client, "get_rig_doctor", json!({ "id": "moth-v1" })).await;
        assert_eq!(read["parts"].as_array().map(Vec::len), Some(2));
        assert_eq!(read["bodyBones"][0], "mixamorig:Spine2");
        let no_part = call_error(
            &client,
            "run_rig_doctor",
            json!({ "id": "moth-v1", "step": "rigid", "parts": [3] }),
        )
        .await;
        assert!(no_part.contains("no part 3"), "{no_part}");
        let no_bone = call_error(
            &client,
            "run_rig_doctor",
            json!({ "id": "moth-v1", "step": "rigid", "parts": [1], "bone": "LeftArm" }),
        )
        .await;
        assert!(no_bone.contains("not a body bone"), "{no_bone}");

        // A real ask starts the bind. With no Blender here it cannot finish:
        // the case says so and the person's draft stays.
        let asked = call(
            &client,
            "run_rig_doctor",
            json!({ "id": "moth-v1", "step": "rigid", "parts": [1, 2], "bone": "Spine2" }),
        )
        .await;
        assert!(asked.is_object(), "{asked}");
        let mut after = read.clone();
        for _ in 0..100 {
            after = call(&client, "get_rig_doctor", json!({ "id": "moth-v1" })).await;
            if after["error"].is_string() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(after["state"], "fixed");
        assert!(after["rigid"].is_null(), "{}", after["rigid"]);
        assert_eq!(after["draft"], read["draft"]);
        let error = after["error"].as_str().unwrap_or_default();
        assert!(
            error.starts_with("The rigid bind did not finish."),
            "{error}"
        );
        server.stop();
    }
}
