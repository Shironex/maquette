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
