//! The Mixamo mark a bake writes into its model, honoured by packs and by
//! Save clips to the library wherever the file ends up.

use std::fs;

use super::tests::humanoid;
use super::*;
use crate::animations::save::{ClipsFrom, plan};
use crate::packs::licence;
use crate::workspace::WorkspaceConfig;

fn mark(mixamo: bool) -> RecipeBakeMark {
    RecipeBakeMark {
        recipe: "wave".into(),
        recipe_version: "0123456789abcdef".into(),
        base_clip: "idle-clip".into(),
        clip: "Idle".into(),
        mixamo,
    }
}

#[test]
fn a_mixamo_bake_is_left_out_of_packs_and_never_saved_to_the_library() {
    let root = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    let inbox = crate::inbox::inbox_dir(root.path(), &config);
    fs::create_dir_all(&inbox).expect("inbox");
    let from = ClipsFrom {
        project: None,
        asset: None,
        inbox: Some("baked.glb".into()),
    };
    let file = inbox.join("baked.glb");

    // Unmarked, its clip is the model's own and could be saved.
    fs::write(&file, humanoid()).expect("model");
    assert!(!licence::marked_mixamo(&file));
    let open = plan(root.path(), &config, &from, false, &[]).expect("plan");
    assert_eq!(open.clips[0].from_library, None);

    write_mark(&file, &mark(true)).expect("mark");
    assert_eq!(mark_of(&file), Some(mark(true)));
    assert!(licence::marked_mixamo(&file), "a pack leaves its clips out");
    let marked = plan(root.path(), &config, &from, false, &[]).expect("plan");
    assert_eq!(
        marked.clips[0].from_library.as_deref(),
        Some("idle-clip"),
        "Save clips skips it"
    );

    // A bake of a clip that is not Mixamo's may be saved.
    fs::write(&file, humanoid()).expect("model");
    write_mark(&file, &mark(false)).expect("mark");
    assert!(!licence::marked_mixamo(&file));
    let free = plan(root.path(), &config, &from, false, &[]).expect("plan");
    assert_eq!(free.clips[0].from_library, None);
}

#[test]
fn an_accepted_mixamo_bake_stays_flagged_after_a_blender_round_trip_drops_its_mark() {
    let p = super::tests::project_with(humanoid());
    let (root, config) = (p.root(), &p.config);
    assert!(!licence::baked_mixamo(root, &p.dir, "elsheria"));

    // A staged bake copied out of the cache and dropped into the inbox.
    let inbox = crate::inbox::inbox_dir(root, config);
    fs::create_dir_all(&inbox).expect("inbox");
    let dropped = inbox.join("baked.glb");
    fs::write(&dropped, humanoid()).expect("model");
    write_mark(&dropped, &mark(true)).expect("mark");
    let accepted = crate::inbox::accept(
        root,
        config,
        &crate::inbox::AcceptRequest {
            item: "baked.glb".into(),
            project: p.id.clone(),
            asset: Some("elsheria".into()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    assert_eq!(accepted.version.version, 2);
    let bakes = crate::recipes::baked::of_asset(&p.dir, "elsheria").expect("bakes");
    assert_eq!(bakes.len(), 1);
    assert_eq!((bakes[0].version, bakes[0].mixamo), (2, true));

    // A Blender step writes the next version without the mark.
    fs::write(p.dir.join("models/elsheria_v3.glb"), humanoid()).expect("v3");
    crate::project::modify(root, &config.layout, &p.id, |m| {
        crate::assets::add_version(m, "elsheria", "models/elsheria_v3.glb".into(), None).map(|_| ())
    })
    .expect("v3");
    assert_eq!(mark_of(&p.dir.join("models/elsheria_v3.glb")), None);

    // Still Mixamo's for packs, and Save clips still skips its clip.
    assert!(licence::baked_mixamo(root, &p.dir, "elsheria"));
    let from = ClipsFrom {
        project: Some(p.id.clone()),
        asset: Some("elsheria".into()),
        inbox: None,
    };
    let plan = plan(root, config, &from, false, &[]).expect("plan");
    assert_eq!(plan.version, Some(3));
    assert_eq!(plan.clips[0].from_library.as_deref(), Some("idle-clip"));
}
