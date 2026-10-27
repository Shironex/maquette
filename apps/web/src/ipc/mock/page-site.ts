/**
 * The pages mock mode shows in the Page view: Frostveil's landing page,
 * as its dev server would serve it, and an HTML page concept beside it.
 * The site page speaks the page bridge (see the root README, Page view):
 * it says its stops, scrolls to the one the scrubber asks for, reports
 * where it is and steady frame times, so screenshots read the same each
 * run. Given the frame proxy's bridge shim instead (`?page=shim`), it has
 * no bridge of its own and carries the shim, as the proxy serves a site
 * without one. `#maquette-poster` stands in for the query a real dev
 * server reads.
 */

export interface SiteImages {
  /** One picture per stop, in tour order; a longer tour reuses them. */
  stops: string[];
  logo?: string;
}

interface SiteStop {
  name: string;
  line: string;
}

const STOPS: SiteStop[] = [
  { name: 'Arrival', line: 'A frozen lake under the aurora, and one swordswoman at its heart.' },
  { name: 'Veil gate', line: 'The gate was a concept sheet on Monday.' },
  { name: 'Crystals', line: 'Every crystal shipped compressed, under budget.' },
  { name: 'Keystone', line: 'Look up: the keystone is one draw.' },
  { name: 'Pullback', line: 'The whole arena, ready for the web.' },
];

/** A long tour (`?page=stops-10`), with long names side by side at the end. */
export const MANY_STOPS: SiteStop[] = [
  { name: 'Arrival', line: 'A frozen lake under the aurora, and one swordswoman at its heart.' },
  { name: 'Veil gate at moonrise', line: 'The gate was a concept sheet on Monday.' },
  { name: 'Lantern path', line: 'Crystal lanterns light the way across the ice.' },
  { name: 'Crystals under the moon', line: 'Every crystal shipped compressed, under budget.' },
  { name: 'Keystone', line: 'Look up: the keystone is one draw.' },
  { name: 'Rune circle', line: 'The circle is a single plane and an emissive map.' },
  { name: 'Academy', line: 'Where the models were made.' },
  { name: 'Guardian statues in a row', line: 'Twelve statues, one material each.' },
  { name: 'Testimonials', line: 'What players said after the first night.' },
  { name: 'Contact', line: 'Join the next playtest.' },
];

const STYLE = `
*{box-sizing:border-box}html,body{margin:0;background:#101420;color:#eef0f6;font:15px/1.5 system-ui,sans-serif}
header{position:fixed;inset:0 0 auto 0;z-index:3;display:flex;align-items:center;gap:22px;padding:16px 28px;background:rgba(16,20,32,.86);border-bottom:1px solid #232a3d;font-size:13px;color:#c3c8d6}
header b{font-size:14px;letter-spacing:.14em;color:#fff;margin-right:auto}
header .cta{background:#c9b8f5;color:#16122a;padding:8px 13px;border-radius:7px;font-weight:600}
.stage{position:fixed;inset:0;z-index:0}
.stage img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;opacity:0;transition:opacity .45s}
.stage img.on{opacity:1}
.stage:after{content:"";position:absolute;inset:0;background:linear-gradient(90deg,rgba(16,20,32,.92) 0,rgba(16,20,32,.55) 38%,rgba(16,20,32,0) 62%)}
section{position:relative;z-index:1;height:100vh;display:flex;flex-direction:column;justify-content:center;padding:0 7vw;max-width:620px}
small{font:600 11px/1 ui-monospace,monospace;letter-spacing:.16em;color:#c9b8f5;text-transform:uppercase}
h1{font-size:46px;line-height:1.05;letter-spacing:-.03em;margin:14px 0 12px}
h2{font-size:32px;line-height:1.1;letter-spacing:-.02em;margin:12px 0 10px}
p{color:#b4bacb;margin:0 0 18px;max-width:380px}
.btns{display:flex;gap:10px}.btns span{padding:10px 15px;border-radius:7px;font-weight:600;font-size:13px}
.btns span:first-child{background:#c9b8f5;color:#16122a}.btns span:last-child{border:1px solid #3b4460}
.hint{position:absolute;bottom:26px;left:7vw;font:500 11px/1 ui-monospace,monospace;color:#7d8499}
.still{position:fixed;top:72px;left:16px;z-index:4;display:none;background:rgba(0,0,0,.7);color:#fff;font-size:11px;padding:5px 9px;border-radius:6px}
.poster .still{display:block}
@media (max-width:600px){header span:not(.cta){display:none}h1{font-size:34px}h2{font-size:26px}section{padding:0 24px}.stage:after{background:linear-gradient(0deg,rgba(16,20,32,.95) 0,rgba(16,20,32,.5) 45%,rgba(16,20,32,0) 70%)}section{justify-content:flex-end;padding-bottom:70px}}
`;

/** The page's own script: the picture for the section in view. */
const PICTURES = `
const sections=[...document.querySelectorAll('section')];
const imgs=[...document.querySelectorAll('.stage img')];
if(location.hash.includes('maquette-poster'))document.documentElement.classList.add('poster');
const progress=()=>{const y=scrollY;let i=0;for(let k=0;k<sections.length;k++)if(sections[k].offsetTop<=y+1)i=k;const s=sections[i],h=s.offsetHeight||1;return Math.min(sections.length-1,i+Math.max(0,Math.min(1,(y-s.offsetTop)/h)))};
const show=()=>{const n=Math.round(progress());imgs.forEach((im,k)=>im.classList.toggle('on',k===n))};
addEventListener('scroll',show,{passive:true});show();
`;

/** The bridge, as a site would add it, with fixed numbers for the screenshots. */
const bridge = (tour: SiteStop[]) => `
const stops=${JSON.stringify(tour.map(s => ({ name: s.name })))};
let origin=null;
const send=m=>{if(origin)parent.postMessage(m,origin)};
const at=()=>{const p=progress();send({maquette:1,type:'at',stop:Math.round(p*1000)/1000,scroll:{top:Math.round(scrollY),max:Math.max(0,document.documentElement.scrollHeight-innerHeight)}})};
addEventListener('scroll',at,{passive:true});
const stats=()=>{const phone=innerWidth<600,poster=document.documentElement.classList.contains('poster');if(poster)return;send({maquette:1,type:'stats',frameMs:phone?21.4:9.4,p95Ms:phone?38.2:12.1,drawCalls:phone?71:86})};
addEventListener('message',e=>{if(e.source!==parent||!e.data||e.data.maquette!==1)return;origin=e.origin&&e.origin!=='null'?e.origin:'*';
if(e.data.type==='hello'){send({maquette:1,type:'ready',stops,features:['goto','at','stats','poster']});at();stats();setInterval(stats,1000)}
else if(e.data.type==='goto'&&typeof e.data.stop==='number'){const s=Math.max(0,Math.min(sections.length-1,e.data.stop)),i=Math.floor(s),sec=sections[i];const top=sec.offsetTop+(s-i)*sec.offsetHeight;scrollTo({top,behavior:e.data.smooth?'smooth':'auto'})}});
`;

/**
 * The mock site's landing page. With `shim` (the frame proxy's bridge
 * shim) the page has no bridge and carries the shim instead.
 */
export function mockSitePage(images: SiteImages, tour: SiteStop[] = STOPS, shim?: string): string {
  const pics = tour
    .map((_, i) => images.stops[i % Math.max(1, images.stops.length)] ?? '')
    .map((src, i) => `<img src="${src}" alt=""${i === 0 ? ' class="on"' : ''}>`)
    .join('');
  const sections = tour
    .map((s, i) =>
      i === 0
        ? `<section><small>Frostveil</small><h1>A frozen lake you can scroll through.</h1><p>${s.line}</p><div class="btns"><span>Play the demo</span><span>How it was made</span></div><span class="hint">Scroll to follow the path</span></section>`
        : `<section><small>Stop ${i + 1}</small><h2>${s.name}</h2><p>${s.line}</p></section>`
    )
    .join('');
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Frostveil</title><style>${STYLE}</style></head><body><header><b>FROSTVEIL</b><span>The world</span><span>Making of</span><span>Journal</span><span class="cta">Wishlist</span></header><div class="stage">${pics}</div><span class="still">Still image, no WebGL</span>${sections}<script>${PICTURES}${shim ? '' : bridge(tour)}</script>${shim ? `<script>${shim}</script>` : ''}</body></html>`;
}

/** A page concept from the Lab, without the bridge: the scrubber has nothing to drive. */
export function mockConceptPage(image: string): string {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Home v2</title><style>body{margin:0;font:15px/1.5 system-ui,sans-serif;background:#16132a;color:#ece8ff}header{display:flex;justify-content:space-between;padding:18px 28px;font-weight:700;letter-spacing:.12em}main{display:grid;grid-template-columns:1fr 1.2fr;gap:28px;padding:40px 28px;align-items:center}h1{font-size:42px;line-height:1.05;margin:0 0 14px}img{width:100%;border-radius:14px}p{color:#a9a3c9}@media (max-width:600px){main{grid-template-columns:1fr}}</style></head><body><header><span>FROSTVEIL</span><span>Concept v2</span></header><main><div><h1>Spend a night on the frozen lake.</h1><p>A page concept: the hero frames the arena and the copy sits on the dark side.</p></div><img src="${image}" alt="The frozen lake at night"></main></body></html>`;
}
