/**
 * The pages mock mode shows in the Page view: Moon Island's landing page,
 * as its dev server would serve it, and an HTML page concept beside it.
 * The site page speaks the page bridge (see the root README, Page view):
 * it says its stops, scrolls to the one the scrubber asks for, reports
 * where it is and steady frame times, so screenshots read the same each
 * run. `#maquette-poster` stands in for the query a real dev server reads.
 */

export interface SiteImages {
  /** One picture per stop, in tour order. */
  stops: string[];
  logo?: string;
}

const STOPS = [
  { name: 'Porch', line: 'A moonlit island, one porch and a tree in bloom.' },
  { name: 'Sakura', line: 'The sakura was a concept sketch on Monday.' },
  { name: 'Shrubs', line: 'Every shrub shipped compressed, under budget.' },
  { name: 'Canopy', line: 'Look up: the canopy is one draw.' },
  { name: 'Pullback', line: 'The whole island, ready for the web.' },
];

const STYLE = `
*{box-sizing:border-box}html,body{margin:0;background:#101420;color:#eef0f6;font:15px/1.5 system-ui,sans-serif}
header{position:fixed;inset:0 0 auto 0;z-index:3;display:flex;align-items:center;gap:22px;padding:16px 28px;background:rgba(16,20,32,.86);border-bottom:1px solid #232a3d;font-size:13px;color:#c3c8d6}
header b{font-size:14px;letter-spacing:.14em;color:#fff;margin-right:auto}
header .cta{background:#f0b6c8;color:#1a1420;padding:8px 13px;border-radius:7px;font-weight:600}
.stage{position:fixed;inset:0;z-index:0}
.stage img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;opacity:0;transition:opacity .45s}
.stage img.on{opacity:1}
.stage:after{content:"";position:absolute;inset:0;background:linear-gradient(90deg,rgba(16,20,32,.92) 0,rgba(16,20,32,.55) 38%,rgba(16,20,32,0) 62%)}
section{position:relative;z-index:1;height:100vh;display:flex;flex-direction:column;justify-content:center;padding:0 7vw;max-width:620px}
small{font:600 11px/1 ui-monospace,monospace;letter-spacing:.16em;color:#f0b6c8;text-transform:uppercase}
h1{font-size:46px;line-height:1.05;letter-spacing:-.03em;margin:14px 0 12px}
h2{font-size:32px;line-height:1.1;letter-spacing:-.02em;margin:12px 0 10px}
p{color:#b4bacb;margin:0 0 18px;max-width:380px}
.btns{display:flex;gap:10px}.btns span{padding:10px 15px;border-radius:7px;font-weight:600;font-size:13px}
.btns span:first-child{background:#f0b6c8;color:#1a1420}.btns span:last-child{border:1px solid #3b4460}
.hint{position:absolute;bottom:26px;left:7vw;font:500 11px/1 ui-monospace,monospace;color:#7d8499}
.still{position:fixed;top:72px;left:16px;z-index:4;display:none;background:rgba(0,0,0,.7);color:#fff;font-size:11px;padding:5px 9px;border-radius:6px}
.poster .still{display:block}
@media (max-width:600px){header span:not(.cta){display:none}h1{font-size:34px}h2{font-size:26px}section{padding:0 24px}.stage:after{background:linear-gradient(0deg,rgba(16,20,32,.95) 0,rgba(16,20,32,.5) 45%,rgba(16,20,32,0) 70%)}section{justify-content:flex-end;padding-bottom:70px}}
`;

/** The bridge, as a site would add it, with fixed numbers for the screenshots. */
const SCRIPT = `
const stops=${JSON.stringify(STOPS.map(s => ({ name: s.name })))};
const sections=[...document.querySelectorAll('section')];
const imgs=[...document.querySelectorAll('.stage img')];
if(location.hash.includes('maquette-poster'))document.documentElement.classList.add('poster');
let origin=null;
const send=m=>{if(origin)parent.postMessage(m,origin)};
const progress=()=>{const y=scrollY;let i=0;for(let k=0;k<sections.length;k++)if(sections[k].offsetTop<=y+1)i=k;const s=sections[i],h=s.offsetHeight||1;return Math.min(sections.length-1,i+Math.max(0,Math.min(1,(y-s.offsetTop)/h)))};
const show=()=>{const at=progress(),n=Math.round(at);imgs.forEach((im,k)=>im.classList.toggle('on',k===n));return at};
const at=()=>{const p=show();send({maquette:1,type:'at',stop:Math.round(p*1000)/1000,scroll:{top:Math.round(scrollY),max:Math.max(0,document.documentElement.scrollHeight-innerHeight)}})};
addEventListener('scroll',at,{passive:true});show();
const stats=()=>{const phone=innerWidth<600,poster=document.documentElement.classList.contains('poster');if(poster)return;send({maquette:1,type:'stats',frameMs:phone?21.4:9.4,p95Ms:phone?38.2:12.1,drawCalls:phone?71:86})};
addEventListener('message',e=>{if(e.source!==parent||!e.data||e.data.maquette!==1)return;origin=e.origin&&e.origin!=='null'?e.origin:'*';
if(e.data.type==='hello'){send({maquette:1,type:'ready',stops,features:['goto','at','stats','poster']});at();stats();setInterval(stats,1000)}
else if(e.data.type==='goto'&&typeof e.data.stop==='number'){const s=Math.max(0,Math.min(sections.length-1,e.data.stop)),i=Math.floor(s),sec=sections[i];const top=sec.offsetTop+(s-i)*sec.offsetHeight;scrollTo({top,behavior:e.data.smooth?'smooth':'auto'})}});
`;

export function mockSitePage(images: SiteImages): string {
  const pics = images.stops
    .map((src, i) => `<img src="${src}" alt=""${i === 0 ? ' class="on"' : ''}>`)
    .join('');
  const sections = STOPS.map((s, i) =>
    i === 0
      ? `<section><small>Moon Island</small><h1>An island you can scroll through.</h1><p>${s.line}</p><div class="btns"><span>Visit the island</span><span>How it was made</span></div><span class="hint">Scroll to follow the path</span></section>`
      : `<section><small>Stop ${i + 1}</small><h2>${s.name}</h2><p>${s.line}</p></section>`
  ).join('');
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Moon Island</title><style>${STYLE}</style></head><body><header><b>MOON ISLAND</b><span>The island</span><span>Making of</span><span>Journal</span><span class="cta">Plan a visit</span></header><div class="stage">${pics}</div><span class="still">Still image, no WebGL</span>${sections}<script>${SCRIPT}</script></body></html>`;
}

/** A page concept from the Lab, without the bridge: the scrubber has nothing to drive. */
export function mockConceptPage(image: string): string {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Home v2</title><style>body{margin:0;font:15px/1.5 system-ui,sans-serif;background:#f4efe6;color:#2a2520}header{display:flex;justify-content:space-between;padding:18px 28px;font-weight:700;letter-spacing:.12em}main{display:grid;grid-template-columns:1fr 1.2fr;gap:28px;padding:40px 28px;align-items:center}h1{font-size:42px;line-height:1.05;margin:0 0 14px}img{width:100%;border-radius:14px}p{color:#6b6257}@media (max-width:600px){main{grid-template-columns:1fr}}</style></head><body><header><span>MOON ISLAND</span><span>Concept v2</span></header><main><div><h1>Spend a night on the island.</h1><p>A page concept: the hero frames the porch and the copy sits on the dark side.</p></div><img src="${image}" alt="The island at night"></main></body></html>`;
}
