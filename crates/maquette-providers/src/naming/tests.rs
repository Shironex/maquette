//! Tests for writing and reading export names in both forms.

use super::*;

fn name(project: Option<&str>, asset: &str, version: u32, provider: ProviderId) -> ExportName {
    ExportName {
        project: project.map(str::to_string),
        asset: asset.into(),
        version,
        provider,
    }
}

/// How a Maquette from before the project was part of the name read one,
/// kept here to show what it makes of the names written now.
fn older_parse(name: &str) -> Option<(String, u32)> {
    let stem = bare(name);
    let rest = stem.strip_prefix(PREFIX)?;
    let (head, tail) = ProviderId::ALL.iter().find_map(|&service| {
        let word = format!("_{}", provider_word(service));
        let at = rest.rfind(&word)?;
        Some((&rest[..at], &rest[at + word.len()..]))
    })?;
    if !is_copy_suffix(tail) {
        return None;
    }
    let (asset, version) = head.rsplit_once("_v")?;
    let version = version.parse::<u32>().ok()?;
    is_part(asset).then(|| (asset.to_string(), version))
}

#[test]
fn the_stem_carries_the_project_the_asset_the_send_number_and_the_provider() {
    assert_eq!(
        export_stem("moon-island", "crate-lid-3", "crate-lid", ProviderId::Tripo),
        "maq_moon-island_crate-lid_v3_tripo"
    );
    assert_eq!(
        export_stem("studio", "lamp-1", "lamp", ProviderId::Meshy),
        "maq_studio_lamp_v1_meshy"
    );
    assert_eq!(send_number("lamp-12", "lamp"), Some(12));
    assert_eq!(send_number("lamp-x", "lamp"), None);
    assert_eq!(send_number("lamps-1", "lamp"), None);
    assert_eq!(send_number("lamp-", "lamp"), None);
    assert_eq!(send_number("lamp-+1", "lamp"), None, "digits only");
    // The second asset named Lamp is `lamp-2`; its sends are not the first's.
    assert_eq!(send_number("lamp-2-1", "lamp"), None);
    assert_eq!(send_number("lamp-2-1", "lamp-2"), Some(1));
}

#[test]
fn two_projects_sending_the_same_asset_get_two_names() {
    let first = export_stem("moon-island", "desk-lamp-1", "desk-lamp", ProviderId::Tripo);
    let second = export_stem("lantern-bay", "desk-lamp-1", "desk-lamp", ProviderId::Tripo);
    assert_eq!(first, "maq_moon-island_desk-lamp_v1_tripo");
    assert_eq!(second, "maq_lantern-bay_desk-lamp_v1_tripo");
    assert_ne!(first, second);
    // And neither is the name the same send had before projects were in it.
    let older = legacy_stem("desk-lamp-1", "desk-lamp", ProviderId::Tripo);
    assert_eq!(older, "maq_desk-lamp_v1_tripo");
    assert!(first != older && second != older);
}

#[test]
fn both_forms_read_back_with_any_extension_case_and_copy_number() {
    let newer = name(Some("moon-island"), "crate-lid", 3, ProviderId::Tripo);
    for file in [
        "maq_moon-island_crate-lid_v3_tripo.glb",
        "MAQ_Moon-Island_Crate-Lid_V3_Tripo.GLB",
        "maq_moon-island_crate-lid_v3_tripo (1).glb",
        "maq_moon-island_crate-lid_v3_tripo-2.zip",
        "maq_moon-island_crate-lid_v3_tripo 2.fbx",
        "maq_moon-island_crate-lid_v3_tripo_4.glb",
        "/Users/me/Downloads/maq_moon-island_crate-lid_v3_tripo.glb",
        "C:\\Users\\me\\Downloads\\maq_moon-island_crate-lid_v3_tripo.glb",
        "maq_moon-island_crate-lid_v3_tripo",
    ] {
        assert_eq!(parse(file), Some(newer.clone()), "{file}");
    }
    assert_eq!(newer.stem(), "maq_moon-island_crate-lid_v3_tripo");

    let older = name(None, "crate-lid", 3, ProviderId::Tripo);
    for file in [
        "maq_crate-lid_v3_tripo.glb",
        "MAQ_Crate-Lid_V3_Tripo.GLB",
        "maq_crate-lid_v3_tripo (1).glb",
        "maq_crate-lid_v3_tripo-2.zip",
        "maq_crate-lid_v3_tripo 2.fbx",
        "/Users/me/Downloads/maq_crate-lid_v3_tripo.glb",
        "C:\\Users\\me\\Downloads\\maq_crate-lid_v3_tripo.glb",
        "maq_crate-lid_v3_tripo",
    ] {
        assert_eq!(parse(file), Some(older.clone()), "{file}");
    }
    assert_eq!(older.stem(), "maq_crate-lid_v3_tripo");
    let meshy = parse("maq_lamp_v12_meshy.glb").expect("meshy");
    assert_eq!(meshy.provider, ProviderId::Meshy);
    assert_eq!(meshy.version, 12);
}

#[test]
fn other_names_are_not_read_as_exports() {
    for file in [
        "crate-lid-3.glb",
        "model (3).glb",
        "maq_crate-lid_tripo.glb",
        "maq__v3_tripo.glb",
        "maq_moon__v3_tripo.glb",
        "maq__crate_v3_tripo.glb",
        "maq_crate lid_v3_tripo.glb",
        "maq_crate-lid_v3_blender.glb",
        "maq_crate-lid_v3_tripo_final.glb",
        "maq_crate-lid_vx_tripo.glb",
        "maq_crate-lid_v_tripo.glb",
        "maq_crate-lid_v+3_tripo.glb",
        "maq_a_b_c_v3_tripo.glb",
        "maq_moon.island_crate_v3_tripo.glb",
        "tripo_crate.zip",
    ] {
        assert_eq!(parse(file), None, "{file}");
    }
}

#[test]
fn ids_with_dashes_digits_and_provider_words_read_without_a_guess() {
    // (project, asset, number, provider): every one must come back as written.
    let cases = [
        ("a", "b", 1, ProviderId::Tripo),
        ("moon-island-2", "desk-lamp-2", 7, ProviderId::Meshy),
        // Parts that look like the version marker.
        ("v1", "v2", 3, ProviderId::Tripo),
        ("p", "v2-lamp", 7, ProviderId::Meshy),
        ("v2", "lamp", 1, ProviderId::Tripo),
        // Parts named after a provider, sent to either one.
        ("tripo", "meshy", 1, ProviderId::Tripo),
        ("meshy", "tripo", 1, ProviderId::Meshy),
        ("p", "tripo-statue", 2, ProviderId::Meshy),
        ("p", "meshy-statue", 2, ProviderId::Tripo),
        ("tripo-tests", "tripo", 4, ProviderId::Tripo),
        // Digits everywhere.
        ("2026", "7", 12, ProviderId::Tripo),
        ("p", "lamp-1", 1, ProviderId::Tripo),
    ];
    for (project, asset, version, provider) in cases {
        let job = format!("{asset}-{version}");
        let stem = export_stem(project, &job, asset, provider);
        assert_eq!(
            parse(&format!("{stem}.glb")),
            Some(name(Some(project), asset, version, provider)),
            "{stem}"
        );
        let older = legacy_stem(&job, asset, provider);
        assert_eq!(
            parse(&format!("{older} (2).zip")),
            Some(name(None, asset, version, provider)),
            "{older}"
        );
    }
}

#[test]
fn the_two_forms_never_read_as_each_other() {
    // One part before the number is the older form, two the newer: an
    // asset id holds no `_`, so no older name has two.
    assert_eq!(
        parse("maq_v2_v3_tripo.glb"),
        Some(name(None, "v2", 3, ProviderId::Tripo))
    );
    assert_eq!(
        parse("maq_v1_v2_v3_tripo.glb"),
        Some(name(Some("v1"), "v2", 3, ProviderId::Tripo))
    );
    // The same send under both forms is two names.
    let newer = export_stem("moon", "lamp-1", "lamp", ProviderId::Tripo);
    let older = legacy_stem("lamp-1", "lamp", ProviderId::Tripo);
    assert!(!is_named(&format!("{older}.glb"), &newer));
    assert!(!is_named(&format!("{newer}.glb"), &older));
}

#[test]
fn an_older_maquette_does_not_take_a_new_name_for_one_of_its_own() {
    assert_eq!(
        older_parse("maq_crate-lid_v3_tripo (1).glb"),
        Some(("crate-lid".to_string(), 3)),
        "the stand in reads what it used to"
    );
    let long = "long-project-name-".repeat(5);
    for (project, asset) in [
        ("moon-island", "crate-lid"),
        ("a", "b"),
        ("v1", "v2"),
        ("tripo", "meshy"),
        ("A Project_copy", "lamp"),
        (long.as_str(), "lamp"),
    ] {
        let stem = export_stem(project, &format!("{asset}-1"), asset, ProviderId::Tripo);
        assert_eq!(older_parse(&format!("{stem}.glb")), None, "{stem}");
    }
}

#[test]
fn a_plain_id_is_its_own_part_and_a_long_or_odd_one_is_cut_with_a_tag() {
    assert_eq!(project_part("moon-island"), "moon-island");
    assert_eq!(project_part("a"), "a");
    let exact = "a".repeat(PROJECT_PART_MAX);
    assert_eq!(project_part(&exact), exact, "the cap itself fits");

    let long = "the-very-long-name-of-a-project-alpha";
    let cut = project_part(long);
    assert_eq!(cut.len(), PROJECT_PART_MAX, "{cut}");
    assert!(cut.starts_with("the-very-long-na--"), "{cut}");
    assert!(is_part(&cut), "{cut}");
    // The same id gives the same part every time, on every computer.
    assert_eq!(cut, project_part(long));
    assert_eq!(cut, "the-very-long-na--8ecd3d");
    // Two long ids that start alike are told apart by the tag.
    let other = project_part("the-very-long-name-of-a-project-beta");
    assert!(other.starts_with("the-very-long-na--") && other != cut);

    // A project folder named by hand: spaces, capitals, `_`, a dot.
    for odd in ["Moon_Island v2", "moon.island", "MOON", "a--b", "-a", "a-"] {
        let part = project_part(odd);
        assert!(is_part(&part) && part.contains(TAG_MARK), "{odd}: {part}");
        assert!(part.len() <= PROJECT_PART_MAX, "{odd}: {part}");
    }
    assert!(project_part("Moon_Island v2").starts_with("moon-island-v2--"));
    // `moon_island` and `moon-island` are two folders, so two parts.
    assert_ne!(project_part("moon_island"), project_part("moon-island"));
    // Nothing a slug keeps: the tag still tells them apart.
    let snow = project_part("\u{96ea}");
    let moon = project_part("\u{6708}");
    assert!(snow.starts_with("project--") && moon.starts_with("project--"));
    assert_ne!(snow, moon);

    // A cut part never reads as a plain id: no slug holds the mark.
    assert!(!is_plain(&cut));
    assert_eq!(
        project_part(&cut).len(),
        PROJECT_PART_MAX,
        "a folder named like a cut part is cut again"
    );
    assert_ne!(project_part(&cut), cut);
}

#[test]
fn the_longest_name_stays_short_and_safe_on_windows() {
    let project = "p".repeat(60);
    let asset = format!("{}-12", "a".repeat(60));
    let job = format!("{asset}-{}", u32::MAX);
    let stem = export_stem(&project, &job, &asset, ProviderId::Tripo);
    let longest = PREFIX.len() + PROJECT_PART_MAX + 1 + ASSET_PART_MAX + 2 + 10 + 1 + 5;
    assert_eq!(stem.len(), longest, "{stem}");
    assert!(longest <= 90);
    assert!(
        stem.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'),
        "{stem}"
    );
    // It still reads back, as the parts it was written with.
    let read = parse(&format!("{stem} (1).glb")).expect("reads back");
    assert_eq!(
        read.project.as_deref(),
        Some(project_part(&project).as_str())
    );
    assert_eq!(read.asset, asset_part(&asset));
    assert_eq!(read.version, u32::MAX);
    assert!(is_named(&format!("{stem}.zip"), &stem));

    // An asset within its cap keeps its whole id.
    assert_eq!(asset_part("iron-street-lamp"), "iron-street-lamp");
    assert_eq!(asset_part(&asset).len(), ASSET_PART_MAX);
    // A name Windows keeps for devices is never the whole stem.
    let reserved = export_stem("con", "nul-1", "nul", ProviderId::Tripo);
    assert_eq!(reserved, "maq_con_nul_v1_tripo");
}

#[test]
fn a_file_is_named_for_a_send_under_any_case_extension_and_copy_number() {
    let expected = "maq_moon-island_crate-lid_v3_tripo";
    for file in [
        "maq_moon-island_crate-lid_v3_tripo.glb",
        "MAQ_MOON-ISLAND_CRATE-LID_V3_TRIPO.ZIP",
        "maq_moon-island_crate-lid_v3_tripo (1).glb",
        "maq_moon-island_crate-lid_v3_tripo-2.glb",
        "maq_moon-island_crate-lid_v03_tripo.glb",
        "folder/maq_moon-island_crate-lid_v3_tripo.glb",
    ] {
        assert!(is_named(file, expected), "{file}");
    }
    for file in [
        "maq_crate-lid_v3_tripo.glb",
        "maq_moon-island_crate-lid_v30_tripo.glb",
        "maq_moon-island_crate-lid_v3_meshy.glb",
        "maq_lantern-bay_crate-lid_v3_tripo.glb",
        "maq_moon-island_crate_v3_tripo.glb",
        "crate-lid-3.glb",
    ] {
        assert!(!is_named(file, expected), "{file}");
    }
    assert!(!is_named("maq_a_v1_tripo.glb", ""), "no name, no match");
    // The older form, as its send was handed out.
    assert!(is_named(
        "maq_crate-lid_v3_tripo (1).glb",
        "maq_crate-lid_v3_tripo"
    ));
    // A name with no readable number still matches itself.
    let odd = legacy_stem("lamp-x", "lamp", ProviderId::Tripo);
    assert_eq!(odd, "maq_lamp_v-x_tripo");
    assert!(is_named("maq_lamp_v-x_tripo (1).glb", &odd));
}
