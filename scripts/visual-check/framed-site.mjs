/**
 * A dev server that forbids framing, as most production sites do, and the
 * real frame proxy in front of it, for the Page view's framed shot
 * (scripts/visual-check/page.mjs). The server sends
 * `Content-Security-Policy: ...; frame-ancestors 'none'` and
 * `X-Frame-Options: DENY` on every response, speaks the page bridge and
 * opens an HMR style WebSocket, so the shot shows the page, its stops and
 * the socket all coming through the proxy. The proxy is the
 * `frame_proxy` example of maquette-core, run with cargo; no shell.
 */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '../..');

export const FORBIDDING_CSP =
  "default-src 'self' 'unsafe-inline'; frame-ancestors 'none'; img-src 'self' data:; connect-src 'self' ws:";

const STOPS = ['Arrival', 'Courtyard', 'Reading room', 'Garden', 'Contact'];

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Framed site</title><style>
*{box-sizing:border-box}html,body{margin:0;background:#14161c;color:#eef0f6;font:15px/1.5 system-ui,sans-serif}
header{position:fixed;inset:0 0 auto 0;z-index:2;display:flex;gap:18px;align-items:center;padding:16px 28px;background:rgba(20,22,28,.9);border-bottom:1px solid #2a2e3a;font-size:13px;color:#c3c8d6}
header b{margin-right:auto;letter-spacing:.14em;color:#fff}
section{height:100vh;display:flex;flex-direction:column;justify-content:center;padding:0 8vw;max-width:760px}
section:nth-child(odd){background:linear-gradient(120deg,#1b2030,#14161c)}
small{font:600 11px/1 ui-monospace,monospace;letter-spacing:.16em;color:#9fd3c7;text-transform:uppercase}
h1{font-size:44px;line-height:1.05;margin:14px 0 12px}h2{font-size:32px;margin:12px 0 10px}
p{color:#b4bacb;max-width:520px}code{font:13px ui-monospace,monospace;color:#f0c8a0}
.socket{display:inline-block;margin-top:8px;padding:7px 11px;border-radius:7px;background:#232838;font:12px ui-monospace,monospace}
.socket.on{background:#1f3a33;color:#9fd3c7}
</style></head><body>
<header><b>FRAMED SITE</b><span>Sent with frame-ancestors 'none'</span><span>X-Frame-Options: DENY</span></header>
<main>${STOPS.map((name, i) =>
  i === 0
    ? `<section><small>Stop 1</small><h1>This site forbids framing.</h1><p>Every response carries <code>frame-ancestors 'none'</code> and <code>X-Frame-Options: DENY</code>. Maquette's frame proxy drops only those, so the page shows here unchanged.</p><span class="socket" id="socket">HMR socket: connecting</span></section>`
    : `<section><small>Stop ${i + 1}</small><h2>${name}</h2><p>Scroll and the scrubber follows, through the page bridge.</p></section>`
).join('')}</main>
<script>
const stops=${JSON.stringify(STOPS.map(name => ({ name })))};
const sections=[...document.querySelectorAll('section')];
let origin=null;
const send=m=>{if(origin)parent.postMessage(m,origin)};
const progress=()=>{const y=scrollY;let i=0;for(let k=0;k<sections.length;k++)if(sections[k].offsetTop<=y+1)i=k;const s=sections[i],h=s.offsetHeight||1;return Math.min(sections.length-1,i+Math.max(0,Math.min(1,(y-s.offsetTop)/h)))};
const at=()=>send({maquette:1,type:'at',stop:Math.round(progress()*1000)/1000,scroll:{top:Math.round(scrollY),max:Math.max(0,document.documentElement.scrollHeight-innerHeight)}});
addEventListener('scroll',at,{passive:true});
const stats=()=>send({maquette:1,type:'stats',frameMs:8.3,p95Ms:11.6,drawCalls:42});
addEventListener('message',e=>{if(e.source!==parent||!e.data||e.data.maquette!==1)return;origin=e.origin&&e.origin!=='null'?e.origin:'*';
if(e.data.type==='hello'){send({maquette:1,type:'ready',stops,features:['goto','at','stats']});at();stats();setInterval(stats,1000)}
else if(e.data.type==='goto'&&typeof e.data.stop==='number'){const s=Math.max(0,Math.min(sections.length-1,e.data.stop)),i=Math.floor(s),sec=sections[i];scrollTo({top:sec.offsetTop+(s-i)*sec.offsetHeight,behavior:'auto'})}});
const socket=new WebSocket('ws://'+location.host+'/hmr');
socket.onmessage=e=>{const el=document.getElementById('socket');el.textContent='HMR socket: '+e.data+' through '+location.host;el.classList.add('on')};
</script></body></html>`;

/** One unmasked text frame, as a server sends it. */
function textFrame(text) {
  const payload = Buffer.from(text);
  return Buffer.concat([Buffer.from([0x81, payload.length]), payload]);
}

/** Starts the forbidding dev server on a free port; resolves to its address and a stop. */
export function startForbiddingSite() {
  const server = createServer((req, res) => {
    res.setHeader('Content-Security-Policy', FORBIDDING_CSP);
    res.setHeader('X-Frame-Options', 'DENY');
    res.setHeader('Content-Type', 'text/html; charset=utf-8');
    res.end(PAGE);
  });
  // Upgraded sockets leave the server's books, so a stop ends them itself.
  const sockets = new Set();
  server.on('upgrade', (req, socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    const key = req.headers['sec-websocket-key'] ?? '';
    const accept = createHash('sha1')
      .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
      .digest('base64');
    socket.write(
      `HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`
    );
    socket.write(textFrame('connected'));
    socket.on('error', () => {});
  });
  return new Promise((done, fail) => {
    server.once('error', fail);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      done({
        url: `http://localhost:${port}/`,
        stop: () =>
          new Promise(closed => {
            for (const socket of sockets) socket.destroy();
            server.closeAllConnections();
            server.close(() => closed());
          }),
      });
    });
  });
}

/**
 * Starts the frame proxy in front of `upstream`; resolves to its address
 * and a stop once it prints it. Builds the example the first time.
 */
export function startFrameProxy(upstream) {
  const child = spawn(
    'cargo',
    ['run', '-q', '-p', 'maquette-core', '--example', 'frame_proxy', '--', upstream],
    { cwd: root, stdio: ['pipe', 'pipe', 'inherit'] }
  );
  return new Promise((done, fail) => {
    let out = '';
    child.once('error', fail);
    child.once('exit', code => fail(new Error(`the frame proxy exited (${code})`)));
    child.stdout.on('data', chunk => {
      out += chunk;
      const line = out.split('\n')[0]?.trim();
      if (!out.includes('\n') || !line) return;
      done({
        url: line,
        stop: () =>
          new Promise(closed => {
            child.removeAllListeners('exit');
            child.once('exit', () => closed());
            child.stdin.end();
          }),
      });
    });
  });
}

/** Fails unless `url` sends both headers and `proxy` sends neither. */
export async function checkHeaders(url, proxy) {
  const direct = await fetch(url);
  const framed = await fetch(proxy);
  const csp = framed.headers.get('content-security-policy');
  const problems = [
    direct.headers.get('x-frame-options') !== 'DENY' && 'the site does not send X-Frame-Options',
    !direct.headers.get('content-security-policy')?.includes('frame-ancestors') &&
      'the site does not send frame-ancestors',
    framed.headers.get('x-frame-options') && 'the proxy kept X-Frame-Options',
    csp?.includes('frame-ancestors') && 'the proxy kept frame-ancestors',
    csp !== FORBIDDING_CSP.replace(" frame-ancestors 'none';", '') &&
      `the proxy changed the other directives: ${csp}`,
  ].filter(Boolean);
  if (problems.length) throw new Error(`framed check: ${problems.join('; ')}`);
}
