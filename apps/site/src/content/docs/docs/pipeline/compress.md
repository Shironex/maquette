---
title: Compress
description: 'Runs one gltf-transform chain per model: KTX2 textures and meshopt geometry, from the presets you set per model.'
sidebar:
  label: Compress
---

Step 4 of 6 in a new project. Kind: `gltf-transform`, with `toktx` for the textures.

## What it does

For each model in `web/raw/`, Compress writes your material tweaks into a copy under `cache/`, then runs one texture pass per codec group and one geometry pass. The raw export never changes.

The step reruns when a preset or a tweak changes, because both are part of its cache key.

## Inputs and outputs

**Inputs**

- `web/raw/*.glb` from [Export GLB](/docs/pipeline/export-glb/)
- The presets per model
- The `materials.json` tweaks

**Outputs**

- `web/dist/*.glb`, one per model
- The raw and shipped size per model, in the log
- Compress removes every model in `web/dist` that no current model makes, and logs each one.

## Settings

Set them in the Pipeline view's Compression presets panel, or with [`set_compression_presets`](/docs/agents/mcp-tools/#set_compression_presets). Only the fields you give change. A model with no row uses the workspace defaults.

| Setting            | Values                  | Notes                                                                                          |
| ------------------ | ----------------------- | ---------------------------------------------------------------------------------------------- |
| `textureSize`      | 512, 1024, 2048, 4096   | The longest side of every texture, in px.                                                      |
| `dataTextureSize`  | 512 to 4096, or `null`  | A smaller size for the normal, roughness, metal and occlusion maps. `null` uses `textureSize`. |
| `codec`            | `etc1s`, `uastc`        | ETC1S is small. UASTC keeps more detail and costs bytes.                                       |
| `mapCodecs.color`  | `etc1s`, `uastc`        | Base color and emissive. Follows `codec` when left out.                                        |
| `mapCodecs.normal` | `etc1s`, `uastc`        | Normal maps. Default UASTC, since ETC1S blocks show as banding in the lighting.                |
| `mapCodecs.other`  | `etc1s`, `uastc`        | Metal, roughness, occlusion and the rest. Follows `codec` when left out.                       |
| `meshopt`          | `low`, `medium`, `high` | Low only quantizes the geometry. High can shift skinned meshes.                                |
| `lock`             | a reason                | A locked row keeps its values and shows the reason.                                            |

The per map group codecs are set in the Inspector, or with [`set_material_tweaks`](/docs/agents/mcp-tools/#set_material_tweaks) under `codecs`. Each group that differs gets its own texture pass, and a model without normal maps skips that pass.

:::note[UASTC normal maps cost bytes]
On a 1K test prop the shipped file went from about 360 KB with all ETC1S to 1.1 to 1.4 MB with UASTC normal maps. A prop that does not need sharp normals can set its normal group to ETC1S.
:::

Without `toktx` on your PATH the texture passes are skipped and the textures stay PNG or JPEG. The log says so.

## Example

```json title="set_compression_presets"
{
  "project": "moon-island",
  "presets": [{ "model": "vending", "textureSize": 1024, "meshopt": "medium" }]
}
```

## Related

- [`pipeline.json`](/docs/pipeline/pipeline-json/#compress-steps) holds the step's `compress` folders and the presets rows.
- [`get_pipeline`](/docs/agents/mcp-tools/#get_pipeline) shows the presets and model sizes.
- [`get_material_tweaks`](/docs/agents/mcp-tools/#get_material_tweaks) and [`set_material_tweaks`](/docs/agents/mcp-tools/#set_material_tweaks) read and change tint, roughness, metalness, emissive strength and codecs.
