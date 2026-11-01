/** Runs the gate's steps: a lane in order, lanes side by side, every line labelled. */
import { spawn, spawnSync } from 'node:child_process';
import { createWriteStream, mkdirSync } from 'node:fs';
import { basename, join } from 'node:path';

/** What cmd.exe would read as more than plain text. */
const CMD_SPECIAL = /["%^&|<>!()]/;
/** The lines of a failed step the summary prints again, since they have scrolled away. */
const TAIL_LINES = 60;
/** How long a stopped step gets to end by itself. */
const STOP_GRACE_MS = 2000;

/**
 * The program and arguments for a command on this machine. pnpm is a `.cmd` on
 * Windows, which Node refuses to spawn without a shell, so it runs from its own
 * JS entry when pnpm started this script, and through cmd.exe otherwise. cargo
 * is a real program everywhere.
 */
export function resolveCommand(cmd, args, env = process.env, platform = process.platform) {
  if (cmd !== 'pnpm') return [cmd, args];
  const entry = env.npm_execpath;
  if (entry && /^pnpm\.c?js$/.test(basename(entry))) return [process.execPath, [entry, ...args]];
  if (platform !== 'win32') return [cmd, args];
  const special = args.find(arg => CMD_SPECIAL.test(arg));
  if (special) throw new Error(`cannot pass ${special} through cmd.exe; run under pnpm`);
  return ['cmd.exe', ['/d', '/s', '/c', 'pnpm', ...args]];
}

/** Calls `onLine` per complete line of a stream, and once for a last unfinished one. */
function eachLine(stream, onLine) {
  let rest = '';
  stream.setEncoding('utf8');
  stream.on('data', chunk => {
    const lines = (rest + chunk).split(/\r?\n/);
    rest = lines.pop() ?? '';
    lines.forEach(onLine);
  });
  stream.on('end', () => {
    if (rest) onLine(rest);
  });
}

const running = new Set();
/** Set by stopAll: nothing may start after it, or it would outlive the stop. */
let stopped = false;

/** Signals a step's whole process group, which may already have ended. */
function signalGroup(pid, signal) {
  try {
    process.kill(-pid, signal);
  } catch {
    // The group has already ended.
  }
}

/**
 * Stops whatever is still running, for a gate that is itself being stopped, with
 * what each step started: pnpm hands off to cargo, and cargo to rustc. A step
 * still to come stays not run, even one that was only waiting for this one's
 * turn. What ignores the request is killed after a short wait, then `done` is called.
 */
export function stopAll(done, graceMs = STOP_GRACE_MS) {
  stopped = true;
  const pids = [...running].map(child => child.pid).filter(pid => pid !== undefined);
  if (process.platform === 'win32') {
    for (const pid of pids) {
      spawnSync('taskkill', ['/pid', String(pid), '/t', '/f'], { stdio: 'ignore' });
    }
    done();
    return;
  }
  for (const pid of pids) signalGroup(pid, 'SIGTERM');
  setTimeout(() => {
    for (const pid of pids) signalGroup(pid, 'SIGKILL');
    done();
  }, graceMs);
}

/** Runs one step to its end. Never rejects: a step that cannot start has failed. */
function runStep(lane, step, { logDir, print, env }) {
  const started = performance.now();
  const logName = `${lane}-${step.name.replace(/[^a-z0-9]+/gi, '-')}.log`;
  const logFile = join(logDir, logName);
  const log = createWriteStream(logFile);
  const tail = [];
  const onLine = line => {
    print(`${lane} ${step.name} | ${line}`);
    log.write(`${line}\n`);
    tail.push(line);
    if (tail.length > TAIL_LINES) tail.shift();
  };
  return new Promise(resolve => {
    const finish = ok => {
      running.delete(child);
      log.end();
      const seconds = (performance.now() - started) / 1000;
      resolve({ lane, name: step.name, status: ok ? 'ok' : 'FAILED', seconds, tail, logFile });
    };
    let child;
    try {
      const [program, args] = resolveCommand(step.cmd, step.args);
      // Its own process group where there are groups, for stopAll to signal whole.
      const detached = process.platform !== 'win32';
      child = spawn(program, args, { stdio: ['ignore', 'pipe', 'pipe'], env, detached });
    } catch (error) {
      onLine(String(error));
      finish(false);
      return;
    }
    running.add(child);
    eachLine(child.stdout, onLine);
    eachLine(child.stderr, onLine);
    child.on('error', error => {
      onLine(`could not start ${step.cmd}: ${error.message}`);
      finish(false);
    });
    // `close`, not `exit`: the streams have drained, so no line is lost. A
    // signal leaves code null, which is a failure too.
    child.on('close', code => finish(code === 0));
  });
}

/** Runs `work` once every earlier call has finished, so these never overlap. */
function oneAtATime() {
  let last = Promise.resolve();
  return work => {
    const result = last.then(work);
    last = result;
    return result;
  };
}

/**
 * Runs a lane's steps in order. As with `&&`, a failed step ends its lane; the
 * steps after it are marked as not run.
 */
async function runLane(lane, steps, options) {
  const results = [];
  for (const step of steps) {
    const notRun = { lane, name: step.name, status: 'not run', seconds: 0, tail: [] };
    if (results.some(r => r.status === 'FAILED')) {
      results.push(notRun);
      continue;
    }
    // Checked at the last moment: a stop may have come while waiting for a turn.
    const run = () => (stopped ? notRun : runStep(lane, step, options));
    results.push(await (step.exclusive ? options.exclusive(run) : run()));
  }
  return results;
}

/**
 * Runs every lane side by side to its end, so a failure in one still reports
 * on the others, with exclusive steps taking turns across lanes. Returns one
 * result per step; a step's time leaves out its wait for a turn.
 */
export async function runLanes(lanes, { logDir, print = console.log, env = process.env }) {
  mkdirSync(logDir, { recursive: true });
  stopped = false;
  const options = { logDir, print, env, exclusive: oneAtATime() };
  const runs = Object.entries(lanes)
    .filter(([, steps]) => steps.length > 0)
    .map(([lane, steps]) => runLane(lane, steps, options));
  return (await Promise.all(runs)).flat();
}

/** The gate's exit code: a stop always shows, even when every step that ran passed. */
export function exitCode(results, stopCode) {
  if (stopCode !== undefined) return stopCode;
  return results.some(r => r.status === 'FAILED') ? 1 : 0;
}

const clock = seconds =>
  seconds < 60
    ? `${seconds.toFixed(1)}s`
    : `${Math.floor(seconds / 60)}m${Math.round(seconds % 60)
        .toString()
        .padStart(2, '0')}s`;

/** The final report: every step with its time, then what failed and how it ended. */
export function summary(results, seconds, notices = [], wasStopped = false) {
  const failed = results.filter(r => r.status === 'FAILED');
  const width = Math.max(...results.map(r => r.name.length), 4);
  const lines = [''];
  for (const r of failed) {
    lines.push(
      `---- ${r.lane} ${r.name}: last lines (all of it in ${r.logFile}) ----`,
      ...r.tail,
      ''
    );
  }
  for (const r of results) {
    const time = r.status === 'not run' ? '' : clock(r.seconds);
    lines.push(`${r.lane.padEnd(5)}${r.name.padEnd(width + 2)}${time.padStart(7)}  ${r.status}`);
  }
  lines.push('', ...notices);
  for (const r of failed) lines.push(`FAILED: ${r.lane} ${r.name} after ${clock(r.seconds)}`);
  const verdict = wasStopped ? 'STOPPED' : failed.length > 0 ? 'FAILED' : 'ok';
  lines.push(`gate: ${verdict} in ${clock(seconds)}`);
  return lines.join('\n');
}
