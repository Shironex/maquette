//! Reading and swapping base color images.

use serde_json::{Value, json};

use super::base_color::{BaseColorSwap, base_color_images, swap_base_color};
use super::container;

const COLOR: &[u8] = b"\xff\xd8color";
const NORMAL: &[u8] = b"\x89PNGnormal";

/// Two materials sharing one base color texture with a normal map, a
/// third with its base color in an outside file, and MToon on the first.
fn model() -> Vec<u8> {
    let mut bin = COLOR.to_vec();
    bin.extend_from_slice(&[0, 0, 0]);
    let normal_at = bin.len();
    bin.extend_from_slice(NORMAL);
    let json = json!({
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": COLOR.len() },
            { "buffer": 0, "byteOffset": normal_at, "byteLength": NORMAL.len() }
        ],
        "images": [
            { "bufferView": 1, "mimeType": "image/png", "name": "Normal" },
            { "bufferView": 0, "mimeType": "image/jpeg", "name": "Color" },
            { "uri": "outside.png", "name": "Outside" }
        ],
        "samplers": [{ "magFilter": 9729 }],
        "textures": [
            { "sampler": 0, "source": 0 },
            { "sampler": 0, "source": 1 },
            { "source": 2 }
        ],
        "materials": [
            {
                "normalTexture": { "index": 0 },
                "pbrMetallicRoughness": { "baseColorTexture": { "index": 1 } },
                "extensions": { "VRMC_materials_mtoon": { "shadeMultiplyTexture": { "index": 1 } } }
            },
            { "pbrMetallicRoughness": { "baseColorTexture": { "index": 1, "texCoord": 0 } } },
            { "pbrMetallicRoughness": { "baseColorTexture": { "index": 2 } } },
            { "pbrMetallicRoughness": {} }
        ]
    });
    container::join(&json, bin).expect("join")
}

fn json_of(bytes: &[u8]) -> Value {
    container::split(bytes).expect("split").0
}

#[test]
fn lists_each_stored_base_color_image_once() {
    let images = base_color_images(&model()).expect("images");
    assert_eq!(
        images.len(),
        1,
        "the shared one, not the normal map or the outside file"
    );
    assert_eq!(images[0].image, 1);
    assert_eq!(images[0].mime, "image/jpeg");
    assert_eq!(images[0].bytes, COLOR);
}

#[test]
fn a_model_without_materials_has_no_base_color_images() {
    let bytes =
        container::join(&json!({ "asset": { "version": "2.0" } }), Vec::new()).expect("join");
    assert!(base_color_images(&bytes).expect("images").is_empty());
}

#[test]
fn a_swap_adds_the_image_and_points_the_base_color_and_shade_at_it() {
    let swap = BaseColorSwap {
        image: 1,
        mime: "image/jpeg".to_string(),
        bytes: b"\xff\xd8softened".to_vec(),
        suffix: " softened".to_string(),
    };
    let out = swap_base_color(&model(), &[swap]).expect("swap");
    let json = json_of(&out);
    assert_eq!(json["images"][3]["name"], "Color softened");
    assert_eq!(json["textures"][3], json!({ "sampler": 0, "source": 3 }));
    let materials = &json["materials"];
    assert_eq!(
        materials[0]["pbrMetallicRoughness"]["baseColorTexture"]["index"],
        3
    );
    assert_eq!(
        materials[0]["extensions"]["VRMC_materials_mtoon"]["shadeMultiplyTexture"]["index"],
        3
    );
    assert_eq!(
        materials[0]["normalTexture"]["index"], 0,
        "the normal map stays"
    );
    assert_eq!(
        materials[1]["pbrMetallicRoughness"]["baseColorTexture"],
        json!({ "index": 3, "texCoord": 0 })
    );
    assert_eq!(
        materials[2]["pbrMetallicRoughness"]["baseColorTexture"]["index"],
        2
    );
    // The original stays in the file and the new one reads back.
    assert_eq!(json["textures"][1], json!({ "sampler": 0, "source": 1 }));
    let images = base_color_images(&out).expect("images");
    assert_eq!(images.len(), 1);
    assert_eq!(
        (images[0].image, images[0].bytes.as_slice()),
        (3, b"\xff\xd8softened".as_slice())
    );
    let (_, bin) = container::split(&out).expect("split");
    assert_eq!(&bin[..COLOR.len()], COLOR);
}

#[test]
fn a_swap_refuses_an_unknown_image_or_format() {
    let swap = |image, mime: &str| BaseColorSwap {
        image,
        mime: mime.to_string(),
        bytes: vec![1, 2, 3],
        suffix: String::new(),
    };
    let err = swap_base_color(&model(), &[swap(9, "image/png")]).expect_err("no image 9");
    assert!(err.to_string().contains("no image 9"), "{err}");
    let err = swap_base_color(&model(), &[swap(1, "image/webp")]).expect_err("webp");
    assert!(err.to_string().contains("PNG or JPEG"), "{err}");
}

#[test]
fn no_swaps_keep_the_file_as_it_reads() {
    let before = model();
    let after = swap_base_color(&before, &[]).expect("swap");
    assert_eq!(json_of(&after), json_of(&before));
}
