/** Tests for scripts/gate/run.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';

import { resolveCommand, runLanes, stopAll, summary } from './run.mjs';

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

  test('exclusive steps never overlap, and the rest still do', async () => {
    const dir = mkdtempSync(join(logDir, 'turns-'));
    const lock = join(dir, 'lock');
    // Fails when another exclusive step holds the lock.
    const script = `const fs = require('node:fs');
      fs.writeFileSync(${JSON.stringify(lock)}, '', { flag: 'wx' });
      setTimeout(() => fs.rmSync(${JSON.stringify(lock)}), 100);`;
    const turn = name => ({
      name,
      cmd: process.execPath,
      args: ['-e', script],
      exclusive: true,
    });
    const results = await run({ a: [turn('one'), turn('two')], b: [turn('three')] });
    assert.deepEqual(
      results.map(r => r.status),
      ['ok', 'ok', 'ok']
    );
  });

  test('a failed exclusive step still hands over its turn', async () => {
    const results = await run({
      a: [{ ...node('bad', 1), exclusive: true }],
      b: [{ ...node('good', 0), exclusive: true }],
    });
    assert.deepEqual(
      results.map(r => r.status),
      ['FAILED', 'ok']
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

describe('stopAll', () => {
  test('kills a step that ignores the request to stop', async () => {
    const stubborn = {
      name: 'stubborn',
      cmd: process.execPath,
      args: [
        '-e',
        "process.on('SIGTERM', () => {}); console.log('up'); setInterval(() => {}, 1000)",
      ],
    };
    let stopped;
    const print = line => {
      if (line.endsWith('| up')) stopped = new Promise(resolve => stopAll(resolve, 100));
    };
    const [result] = await runLanes({ a: [stubborn] }, { logDir, print });
    await stopped;
    assert.equal(result.status, 'FAILED');
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
