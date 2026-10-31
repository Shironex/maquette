/** Tests for scripts/main-thread-commands.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { describe, test } from 'node:test';

import { checkMainThreadCommands, mainThreadCommands } from './main-thread-commands.mjs';

describe('mainThreadCommands', () => {
  test('finds a plain command, past its other attributes', () => {
    const source = `
#[tauri::command]
#[specta::specta]
pub fn image_api_status() -> u32 { 1 }

#[tauri::command]
fn private_one() {}
`;
    assert.deepEqual(mainThreadCommands(source), ['image_api_status', 'private_one']);
  });

  test('passes async fns, the async attribute and other functions', () => {
    const source = `
#[tauri::command]
#[specta::specta]
pub async fn one() {}

#[tauri::command(async)]
#[specta::specta]
pub fn two() {}

pub fn helper() {}
`;
    assert.deepEqual(mainThreadCommands(source), []);
  });
});

test('checkMainThreadCommands names the file and the command', () => {
  const ctx = {
    glob: () => ['a.rs'],
    read: () => '#[tauri::command]\npub fn slow() {}',
  };
  const [violation] = checkMainThreadCommands(ctx);
  assert.equal(violation.file, 'a.rs');
  assert.match(violation.message, /^slow runs on the main thread/);
});
