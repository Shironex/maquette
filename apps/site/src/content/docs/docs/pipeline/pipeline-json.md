---
title: pipeline.json
description: The file that defines a project's pipeline. Every field of the file, its type, default and what it does, with a real example.
sidebar:
  label: pipeline.json
---

Every project has a `pipeline.json` in its folder. It lists the steps the [Pipeline view](/docs/views/pipeline/) runs and the compression presets per model. Maquette writes it when a project is first opened, from the project's template, and the Pipeline view and the MCP tools change it from then on. You can also edit it by hand.

A pipeline with errors cannot be saved or run, so the file on disk always runs. The Pipeline view lists the problems next to the step.

## The file

```json title="pipeline.json"
{
  "version": 1,
  "root": null,
  "steps": [],
  "presets": []
}
```

| Field     | Type             | Default  | What it does                                                                                                                                           |
| --------- | ---------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `version` | number           | required | The file format. It is `1`. A file with a higher number is refused, so an older Maquette never rewrites a newer file.                                  |
| `root`    | string or `null` | `null`   | The folder steps run in and every relative path resolves against. `null` means the project folder. An imported project points it at its source folder. |
| `steps`   | array of steps   | required | The steps, in the order you list them. They run in dependency order, and steps that do not depend on each other keep this order.                       |
| `presets` | array of presets | required | Compression settings per model, used by the step that has a `compress` field. A model with no row uses the defaults from Settings.                     |

A field marked required must be in the file. Use `[]` when a list is empty. A field with a default may be left out or set to `null`.

## Steps

```json title="a step"
{
  "id": "export",
  "name": "Export GLB",
  "kind": "blender",
  "script": "tools/export_web.py",
  "args": [
    "-b",
    "--factory-startup",
    "--python-exit-code",
    "1",
    "blender/scene_baked.blend",
    "--python",
    "{script}",
    "--",
    "--bake",
    "blender/bake",
    "--out",
    "web/raw"
  ],
  "inputs": ["blender/scene_baked.blend", "blender/bake/*"],
  "outputs": ["web/raw/*.glb"],
  "dependsOn": ["bake"]
}
```

| Field       | Type             | Default  | What it does                                                                                                                               |
| ----------- | ---------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `id`        | string           | required | The step's name in the file. Unique, and no spaces or path characters (letters, digits and dashes). `dependsOn` uses it.                   |
| `name`      | string           | required | What the Pipeline view shows. It cannot be empty.                                                                                          |
| `kind`      | string           | required | The tool: `blender`, `shell`, `gltf-transform`, `toktx` or `ffmpeg`. See [Kinds](#kinds).                                                  |
| `command`   | string or `null` | `null`   | The program a `shell` step runs, for example `node`, `python3` or `sh`. Required for `shell`. The other kinds use the tool Maquette found. |
| `script`    | string or `null` | `null`   | The script the step runs, relative to the root. It fills `{script}` in `args`, is part of the cache key and must exist when the step runs. |
| `args`      | array of strings | required | The arguments, one per entry. They are never joined into a shell string, so there is no quoting. See [Placeholders](#placeholders).        |
| `inputs`    | array of strings | required | Globs relative to the root. What the matching files contain is part of the cache key, so a change reruns the step.                         |
| `outputs`   | array of strings | required | Globs relative to the root that must exist after the step. A step with no outputs runs every time, as Ship does.                           |
| `dependsOn` | array of strings | required | The ids of the steps that run first.                                                                                                       |
| `compress`  | object or `null` | `null`   | Makes the step a compress step. See [Compress steps](#compress-steps).                                                                     |

Every path in `script`, `inputs`, `outputs` and `compress` is relative to the root and cannot start with `/` or `\`, or contain `..`.

### Kinds

| Kind             | Needs                                                                                                        |
| ---------------- | ------------------------------------------------------------------------------------------------------------ |
| `blender`        | `-b` (or `--background`) in `args` before the blend file, `--python` and `--`. Blender always runs headless. |
| `shell`          | A `command`.                                                                                                 |
| `gltf-transform` | `args`, or a `compress` field. The only kind that can have `compress`.                                       |
| `toktx`          | `args`.                                                                                                      |
| `ffmpeg`         | `args`.                                                                                                      |

### Placeholders

Each `{name}` in an argument is replaced before the step starts. Any other `{name}` is an error, and so is `{script}` in a step with no `script`.

| Placeholder      | Becomes                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------- |
| `{script}`       | The step's `script`.                                                                                 |
| `{root}`         | The pipeline root, as a full path.                                                                   |
| `{project}`      | The project folder, as a full path.                                                                  |
| `{workspace}`    | The workspace folder.                                                                                |
| `{siteRepo}`     | The site repo from the project's settings. Empty when the project has none.                          |
| `{shipIndex}`    | A file Maquette writes before each run that says which file name each asset ships as. Ship reads it. |
| `{exportOrigin}` | `scene` or `own`: whether Export GLB keeps each model where the layout put it, from the Ship style.  |

### Caching

A step's key hashes its inputs, its script, its resolved args, the tool and its version, and for a compress step its presets. When the key matches the last good run and the outputs are still there, the step is reported as cached and skipped.

### Compress steps

```json title="the Compress step"
{
  "id": "compress",
  "name": "Compress",
  "kind": "gltf-transform",
  "script": null,
  "args": [],
  "inputs": ["web/raw/*.glb"],
  "outputs": ["web/dist/*.glb"],
  "dependsOn": ["export"],
  "compress": { "from": "web/raw", "to": "web/dist" }
}
```

| Field  | Type   | What it does                                                                                   |
| ------ | ------ | ---------------------------------------------------------------------------------------------- |
| `from` | string | The folder of raw GLBs. Each `<from>/<model>.glb` goes through the passes its preset asks for. |
| `to`   | string | The folder the compressed files land in, as `<to>/<model>.glb`.                                |

Both are folders inside the root. The step's `args` come from the presets, so they stay empty. See [Compress](/docs/pipeline/compress/).

## Presets

```json title="a preset"
{
  "model": "character",
  "label": "Character",
  "textureSize": 2048,
  "dataTextureSize": null,
  "codec": "etc1s",
  "mapCodecs": null,
  "meshopt": "medium",
  "lock": "Character stays on medium. High breaks her clips: the upper body spins further every frame."
}
```

| Field             | Type             | Default  | What it does                                                                                                                                        |
| ----------------- | ---------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model`           | string           | required | The GLB file name without `.glb`, for example `character`. A single name, no slashes.                                                               |
| `label`           | string           | required | What the presets table shows.                                                                                                                       |
| `textureSize`     | number           | required | The longest texture edge in pixels, from 64 to 8192. The presets table offers 512 to 4096.                                                          |
| `dataTextureSize` | number or `null` | `null`   | A smaller edge, 64 to 8192, for the normal, roughness, metal and occlusion maps. Without it every map uses `textureSize`.                           |
| `codec`           | string           | required | `etc1s` (small) or `uastc` (more detail, more bytes). Used by every map group that `mapCodecs` does not set.                                        |
| `mapCodecs`       | object or `null` | `null`   | A codec per map group: `color` (base color and emissive), `normal`, `other` (metal, roughness, occlusion and the rest). Each is `etc1s` or `uastc`. |
| `meshopt`         | string           | required | The geometry pass: `low` (quantize only), `medium` or `high`.                                                                                       |
| `lock`            | string or `null` | `null`   | A reason that stops the row from changing. A locked row refuses edits from the table and from MCP, and shows the reason.                            |

Without `mapCodecs`, color and data maps use `codec` and normal maps use `uastc`, since ETC1S blocks show as banding in the lighting. A locked row without `mapCodecs` keeps `codec` for its normal maps too, so its output does not change. A group left out of `mapCodecs` follows `codec`, except `normal`, which falls back to `uastc`.

A model with no row uses the workspace defaults from Settings: texture size (2K), codec (ETC1S) and meshopt level (medium). Locks are set in this file only. Remove the `lock` to edit the row again.

## Checks

Errors stop a save or a run, and warnings only show. These are the checks:

- A step id is a plain name, and two steps cannot share one.
- A step has a name.
- A `blender` step starts headless. A `shell` step has a `command`. A `gltf-transform`, `toktx` or `ffmpeg` step has `args` or a `compress` field.
- Only a `gltf-transform` step has `compress`, and its folders stay inside the root.
- Every placeholder is a known one, and `{script}` needs a `script`.
- Every `script`, input and output path is relative and has no `..`.
- A step does not depend on itself, on a step that does not exist or on a loop of steps.
- Every preset's model is a plain name and its sizes are between 64 and 8192.
- A pipeline with no steps is a warning.

## Related

- [Pipeline view](/docs/views/pipeline/) runs the steps and shows these checks.
- [Your first project](/docs/start/first-project/#04-blender) shows a Bake light step from a new project.
- [`set_compression_presets`](/docs/agents/mcp-tools/#set_compression_presets) changes the presets from an agent. [`get_pipeline`](/docs/agents/mcp-tools/#get_pipeline) reads them.
