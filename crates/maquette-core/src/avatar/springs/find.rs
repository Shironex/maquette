//! Finding the parts that hang off the body. Nothing changes and nothing
//! waits for a yes: the candidates and their sheet are cached per model,
//! keyed by its content, the script and the bone map.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use super::report::FoundFile;
use super::{FoundSprings, JOB_FILE, SCRIPT, SCRIPT_FILE, SpringsInput, invalid, key};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::cache::{self, CacheRecord};
use crate::pipeline::tools::Toolbox;

const FIND_DIR: &str = "find";
const CANDIDATES_FILE: &str = "candidates.json";
const CANDIDATES_SHEET: &str = "candidates.png";
/// The step cache record's name inside the folder (`find.json`).
const STEP: &str = "find";
const KEY_VERSION: &str = "maquette avatar springs find v1";

/// What the script reads in find mode.
#[derive(Debug, Serialize)]
struct Job<'a> {
    mode: &'a str,
    input: String,
    candidates: &'a str,
    sheets: BTreeMap<&'a str, &'a str>,
    bones: &'a BTreeMap<String, String>,
}

fn outputs() -> Vec<String> {
    vec![CANDIDATES_FILE.to_string(), CANDIDATES_SHEET.to_string()]
}

/// `<springs folder>/find`.
pub fn dir(input: &SpringsInput) -> PathBuf {
    input.dir.join(FIND_DIR)
}

fn run_key(input: &SpringsInput) -> CoreResult<String> {
    let bones = serde_json::to_string(&input.bones).unwrap_or_default();
    Ok(key(&[
        KEY_VERSION,
        &input.hash()?,
        &SCRIPT.replace("\r\n", "\n"),
        &bones,
    ]))
}

fn read(dir: &Path, input: &SpringsInput) -> CoreResult<FoundSprings> {
    let file = dir.join(CANDIDATES_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let found: FoundFile = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    Ok(FoundSprings {
        candidates: found.candidates,
        sheet: dir.join(CANDIDATES_SHEET).display().to_string(),
        input: input.glb.display().to_string(),
        height: found.height,
    })
}

/// The candidates found on this exact model, when a run left them intact;
/// `None` when nobody looked yet or the model changed since.
///
/// # Errors
///
/// Fails when the model cannot be read to key the run.
pub fn found(input: &SpringsInput) -> CoreResult<Option<FoundSprings>> {
    let dir = dir(input);
    let wanted = run_key(input)?;
    let fresh = cache::load_record(&dir, STEP)
        .is_some_and(|r| cache::is_fresh(&r, &wanted, &dir, &outputs()));
    if !fresh {
        return Ok(None);
    }
    // A file an older version wrote in another shape is a miss.
    Ok(read(&dir, input).ok())
}

/// Looks for the parts hanging off the body, or reads the last look at
/// this exact model. Blocks while Blender runs.
///
/// # Errors
///
/// Fails without Blender, or when the script refuses the model or leaves
/// an output out.
pub fn find(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    input: &SpringsInput,
) -> CoreResult<FoundSprings> {
    if let Some(found) = found(input)? {
        return Ok(found);
    }
    let dir = dir(input);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    cache::forget(&dir, STEP);
    for old in outputs() {
        let _ = fs::remove_file(dir.join(old));
    }
    let job = Job {
        mode: "find",
        input: input.glb.display().to_string(),
        candidates: CANDIDATES_FILE,
        sheets: BTreeMap::from([("candidates", CANDIDATES_SHEET)]),
        bones: &input.bones,
    };
    write_json(&dir.join(JOB_FILE), &job)?;
    let started = Instant::now();
    let extra = ["--job".to_string(), JOB_FILE.to_string()];
    runner.run_script(tools, &dir, (SCRIPT_FILE, SCRIPT), &extra)?;
    if let Some(missing) = outputs().iter().find(|o| !dir.join(o).is_file()) {
        return Err(invalid(format!(
            "Blender finished without writing {missing}."
        )));
    }
    let found = read(&dir, input)?;
    cache::save_record(
        &dir,
        STEP,
        &CacheRecord {
            key: run_key(input)?,
            outputs: cache::stamp_outputs(&dir, &outputs()),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    )?;
    Ok(found)
}
