/**
 * In-page helpers of capture-screens.mjs: the neutral renaming and the WebP
 * encoder, both run inside the page through `evaluate`.
 */

const WEBP_QUALITY = 0.85;

/** Encodes a PNG screenshot to WebP inside the page, at its own size or scaled to `size`. */
export function toWebp(page, png, size) {
  return page
    .evaluate(
      async ({ data, quality, size }) => {
        const image = new Image();
        image.src = `data:image/png;base64,${data}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = size?.width ?? image.width;
        canvas.height = size?.height ?? image.height;
        const ctx = canvas.getContext('2d');
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
        return canvas.toDataURL('image/webp', quality).split(',')[1];
      },
      { data: png.toString('base64'), quality: WEBP_QUALITY, size }
    )
    .then(b64 => Buffer.from(b64, 'base64'));
}

/** Neutral names for the sample project, as the site has always shown. */
export function rename() {
  const swap = s =>
    s
      .replace(/Frostveil/g, 'Winter Spire')
      .replace(/frostveil site/g, 'winter spire site')
      .replace(/frostveil/g, 'winter-spire')
      .replace(/Elsheria/g, 'Aria')
      .replace(/elsheria/g, 'aria');
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const next = swap(n.nodeValue);
    if (next !== n.nodeValue) n.nodeValue = next;
  }
  for (const el of document.querySelectorAll('input, textarea')) {
    const next = swap(el.value);
    if (next !== el.value) el.value = next;
  }
  for (const el of document.querySelectorAll('[aria-label],[title],[alt]')) {
    for (const a of ['aria-label', 'title', 'alt']) {
      const v = el.getAttribute(a);
      if (v && swap(v) !== v) el.setAttribute(a, swap(v));
    }
  }
}
