//! Removing a recipe around its bake: refused while it bakes or its bake
//! is being added; a staged bake reads out of date while the recipe is in
//! the trash and can be added again once it is back.

use super::tests::{bakes as service, reaching, settled};
use super::*;
use crate::pipeline::tools::Toolbox;
use crate::recipes;

#[test]
fn a_recipe_is_removed_only_while_its_bake_is_idle_and_its_staged_bake_waits() {
    let p = reaching();
    let trash = p.root().join("trash-of-frostveil");
    let (bakes, _) = service(true);
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let remove = || recipes::remove(&p.dir, &trash, &p.recipe.id);
    let baking = bakes.while_idle(p.root(), &p.config, &p.id, &p.recipe.id, remove);
    assert!(
        baking
            .expect_err("baking")
            .to_string()
            .contains("is baking")
    );
    bakes.cancel(&p.id, &p.recipe.id).expect("cancel");
    settled(&bakes, &p);

    let (bakes, _) = service(false);
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let done = settled(&bakes, &p);
    assert_eq!(done.state, RecipeBakeState::Done, "{:?}", done.error);
    let claim = bakes
        .claim_add(p.root(), &p.config, &p.id, &p.recipe.id, done.finished_ms)
        .expect("claim");
    let adding = bakes.while_idle(p.root(), &p.config, &p.id, &p.recipe.id, remove);
    assert!(
        adding
            .expect_err("adding")
            .to_string()
            .contains("being added")
    );
    drop(claim);

    let removed = bakes
        .while_idle(p.root(), &p.config, &p.id, &p.recipe.id, remove)
        .expect("remove");
    assert!(ready_to_add(p.root(), &p.config, &p.id, &p.recipe.id).is_err());
    let stale = record_of(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("record")
        .expect("staged");
    assert!(stale.stale.is_some_and(|why| why.contains("gone")));

    recipes::restore(&p.dir, &trash, &removed.trash).expect("restore");
    ready_to_add(p.root(), &p.config, &p.id, &p.recipe.id).expect("addable again");
}
