/** Tests for scripts/gate/run.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';

import { resolveCommand, runLanes, summary } from './run.mjs';

const logDir = mkdtempSync(join(tmpdir(), 'gate-run-'));
after(() => rmSync(logDir, { recursive: true, force: true }));

const node = (name, code) => ({
  name,
  cmd: process.execPath,
  args: ['-e', `console.log('hello from ${name}'); process.exit(${code})`],
});
const run = lanes => runLanes(lanes, { logDir, print: () => {} });

describe('resolveCommand', () => {
  const pnpmEntry = { npm_execpath: 'C:/pnpm/bin/pnpm.cjs' };

  test('cargo runs as itself', () => {
    assert.deepEqual(resolveCommand('cargo', ['fmt'], {}, 'win32'), ['cargo', ['fmt']]);
  });

  test('pnpm runs from its JS entry when it started the script', () => {
    assert.deepEqual(resolveCommand('pnpm', ['lint'], pnpmEntry, 'win32'), [
      process.execPath,
      [pnpmEntry.npm_execpath, 'lint'],
    ]);
  });

  test('an npm_execpath of another manager is not used', () => {
    const env = { npm_execpath: '/x/npm-cli.js' };
    assert.deepEqual(resolveCommand('pnpm', ['lint'], env, 'darwin'), ['pnpm', ['lint']]);
  });

  test('without it Windows goes through cmd.exe, and refuses what cmd would read', () => {
    assert.deepEqual(resolveCommand('pnpm', ['lint'], {}, 'win32'), [
      'cmd.exe',
      ['/d', '/s', '/c', 'pnpm', 'lint'],
    ]);
    assert.throws(() => resolveCommand('pnpm', ['a&b'], {}, 'win32'), /cmd\.exe/);
  });
});

describe('runLanes', () => {
  test('all green', async () => {
    const results = await run({ a: [node('one', 0), node('two', 0)], b: [node('three', 0)] });
    assert.deepEqual(
      results.map(r => r.status),
      ['ok', 'ok', 'ok']
    );
  });

  test('a failure ends its lane and not the other', async () => {
    const results = await run({
      a: [node('one', 3), node('two', 0)],
      b: [node('three', 0), node('four', 0)],
    });
    assert.deepEqual(
      results.map(r => `${r.name} ${r.status}`),
      ['one FAILED', 'two not run', 'three ok', 'four ok']
    );
  });

  test('a program that cannot start is a failure', async () => {
    const [result] = await run({ a: [{ name: 'nope', cmd: 'no-such-program-here', args: [] }] });
    assert.equal(result.status, 'FAILED');
    assert.match(result.tail.join('\n'), /could not start/);
  });

  test('a killed step is a failure', async () => {
    const step = {
      name: 'killed',
      cmd: process.execPath,
      args: ['-e', "process.kill(process.pid, 'SIGKILL')"],
    };
    const [result] = await run({ a: [step] });
    assert.equal(result.status, 'FAILED');
  });

  test('keeps the output for the log and the summary', async () => {
    const [result] = await run({ a: [node('talks', 1)] });
    assert.deepEqual(result.tail, ['hello from talks']);
    await new Promise(resolve => setTimeout(resolve, 50));
    assert.equal(readFileSync(result.logFile, 'utf8'), 'hello from talks\n');
  });
});

describe('summary', () => {
  test('names the failed step with its time and shows its last lines', async () => {
    const results = await run({ a: [node('bad', 1), node('later', 0)] });
    const text = summary(results, 75, ['NOTICE: a note']);
    assert.match(text, /hello from bad/);
    assert.match(text, /FAILED: a bad after \d/);
    assert.match(text, /later\s+not run/);
    assert.match(text, /NOTICE: a note/);
    assert.match(text, /gate: FAILED in 1m15s/);
  });

  test('says ok when nothing failed', async () => {
    assert.match(summary(await run({ a: [node('good', 0)] }), 5), /gate: ok in 5\.0s/);
  });
});
