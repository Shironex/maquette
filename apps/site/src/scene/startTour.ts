/**
 * The tour's page side: reads the scroll as the stop sections pass over the
 * pinned stage, keeps the stop rail and the posters in step with it, and
 * starts the 3D island on desktop class devices once the page has loaded.
 * No three.js imports; the island comes in through a dynamic import.
 */
import { TOUR } from '../content/copy';
import { startBridge } from './bridge';
import { LAST_STOP, nearestStop, presence, smoothstep } from './progress';
import { trackTour } from './scroll';
import {
  DESKTOP_QUERY,
  FAILED_EVENT,
  READY_EVENT,
  prefersReducedMotion,
  stopAt,
  supportsWebGL2,
} from './tour';

/** Stops take turns as sections from this width up; below it the page is the phone layout. */
const TOURING_QUERY = '(min-width: 900px)';

export function startTour(tour: HTMLElement): void {
  const sections = [...tour.querySelectorAll<HTMLElement>('[data-stop-section]')];
  const tracker = trackTour(tour, sections, window.matchMedia(TOURING_QUERY));
  const reduced = prefersReducedMotion();

  const callouts = tour.querySelector<HTMLElement>('.callouts');
  const panels = sections.map(section => section.querySelector<HTMLElement>('.panel'));
  const fill = tour.querySelector<HTMLElement>('[data-fill]');
  const counter = tour.querySelector('[data-counter]');
  const buttons = [...tour.querySelectorAll<HTMLElement>('[data-goto]')];
  const posters = [...tour.querySelectorAll<HTMLImageElement>('[data-poster]')];
  let current = -1;
  let live = false;

  const loadPoster = (index: number) => {
    const img = posters[index];
    if (img?.dataset.src && !img.getAttribute('src')) img.src = img.dataset.src;
  };
  const showPoster = (index: number) => {
    if (live) return;
    loadPoster(index);
    for (const img of posters)
      img.classList.toggle('is-shown', Number(img.dataset.poster) === index);
  };

  const setStop = (index: number) => {
    if (index === current) return;
    current = index;
    if (counter)
      counter.textContent = index === 0 ? `${LAST_STOP} stops` : `stop ${index} of ${LAST_STOP}`;
    for (const button of buttons) {
      const at = Number(button.dataset.goto);
      button.dataset.state = at < index ? 'passed' : at === index ? 'current' : 'ahead';
      if (at === index) button.setAttribute('aria-current', 'step');
      else button.removeAttribute('aria-current');
    }
    showPoster(index);
  };

  const render = (progress: number) => {
    setStop(nearestStop(progress));
    fill?.style.setProperty('--fill', (progress / LAST_STOP).toFixed(4));
    if (callouts) callouts.style.opacity = presence(progress, 0).toFixed(3);
    // The copy scrolls with the page and dims only as it nears the header
    // and the rail at the edges of the window.
    panels.forEach((panel, i) => {
      if (panel)
        panel.style.opacity = (1 - smoothstep(Math.abs(progress - i), 0.3, 0.5)).toFixed(3);
    });
    // Without the island, the next poster loads before its stop comes up.
    if (!live && progress > 0) loadPoster(Math.ceil(progress));
  };

  let queued = false;
  tracker.subscribe(progress => {
    if (current < 0) return render(progress);
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      render(tracker.read());
    });
  });
  for (const button of buttons) {
    button.addEventListener('click', () =>
      tracker.scrollToStop(Number(button.dataset.goto), !reduced)
    );
  }

  // Inside Maquette's Page view its scrubber drives the tour.
  startBridge({
    stops: [{ name: 'Hero' }, ...TOUR.map(stop => ({ name: stop.title }))],
    read: tracker.read,
    subscribe: tracker.subscribe,
    goto: (stop, smooth) => tracker.scrollToProgress(stop, smooth && !reduced),
  });

  // ?poster (or Maquette's ?maquette-poster) forces the poster version;
  // ?capture=stops|mobile renders bare frames for the poster script,
  // whatever the device and its settings.
  const params = new URLSearchParams(location.search);
  const capture = params.get('capture');
  if (capture) document.documentElement.classList.add('capture');
  const canvas = tour.querySelector<HTMLElement>('[data-canvas]');
  const desktop =
    window.matchMedia(DESKTOP_QUERY).matches &&
    !reduced &&
    !params.has('poster') &&
    !params.has('maquette-poster');
  if (!canvas || !(desktop || capture)) return;

  let unmount: (() => void) | null = null;
  window.addEventListener(READY_EVENT, () => {
    live = true;
    tour.classList.add('is-live');
  });
  window.addEventListener(FAILED_EVENT, () => {
    live = false;
    tour.classList.remove('is-live');
    unmount?.();
    unmount = null;
    showPoster(current);
  });
  // The WebGL probe waits for idle time too: making a context can start the
  // GPU process, and first paint does not need it.
  const start = () => {
    if (!supportsWebGL2()) return Promise.resolve();
    return import('./mount').then(({ mount }) => {
      unmount = mount(canvas, {
        initialStop: stopAt(current),
        scroll: tracker,
        capture: capture === 'mobile' ? 'mobile' : capture ? 'stops' : undefined,
      });
    });
  };
  const idle = () =>
    'requestIdleCallback' in window
      ? window.requestIdleCallback(() => void start(), { timeout: 1500 })
      : globalThis.setTimeout(() => void start(), 200);
  if (document.readyState === 'complete') idle();
  else window.addEventListener('load', idle, { once: true });
}
