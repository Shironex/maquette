import { describe, expect, it } from 'vitest';
import { detectPlatform, platformScript } from './platform';

const AGENTS = {
  mac: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.4 Safari/605.1.15',
  windows:
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  linux: 'Mozilla/5.0 (X11; Linux x86_64; rv:144.0) Gecko/20100101 Firefox/144.0',
  iphone:
    'Mozilla/5.0 (iPhone; CPU iPhone OS 18_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.4 Mobile/15E148 Safari/604.1',
  android:
    'Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36',
};

describe('detectPlatform', () => {
  it('reads the client hint first', () => {
    expect(
      detectPlatform({ userAgentData: { platform: 'macOS' }, userAgent: AGENTS.windows })
    ).toBe('mac');
    expect(detectPlatform({ userAgentData: { platform: 'Windows' }, userAgent: AGENTS.mac })).toBe(
      'windows'
    );
  });

  it('leaves any other hinted platform as other', () => {
    for (const platform of ['Linux', 'Chrome OS', 'Android', 'iOS']) {
      expect(detectPlatform({ userAgentData: { platform }, userAgent: AGENTS.mac })).toBe('other');
    }
  });

  it('falls back to the user agent when the hint is missing or empty', () => {
    expect(detectPlatform({ userAgent: AGENTS.mac })).toBe('mac');
    expect(detectPlatform({ userAgentData: { platform: '' }, userAgent: AGENTS.windows })).toBe(
      'windows'
    );
    expect(detectPlatform({ userAgent: AGENTS.linux })).toBe('other');
  });

  it('treats phones and an iPad asking for the desktop site as other', () => {
    expect(detectPlatform({ userAgent: AGENTS.iphone })).toBe('other');
    expect(detectPlatform({ userAgent: AGENTS.android })).toBe('other');
    expect(detectPlatform({ userAgent: AGENTS.mac, maxTouchPoints: 5 })).toBe('other');
  });

  it('says other when there is nothing to go on', () => {
    expect(detectPlatform({})).toBe('other');
  });
});

describe('platformScript', () => {
  it('runs on its own and sets data-os on the root element', () => {
    const root = { dataset: {} as Record<string, string> };
    const run = new Function('document', 'navigator', platformScript());
    run({ documentElement: root }, { userAgent: AGENTS.windows });
    expect(root.dataset.os).toBe('windows');
  });

  it('leaves the page neutral when reading the navigator throws', () => {
    const root = { dataset: {} as Record<string, string> };
    const run = new Function('document', 'navigator', platformScript());
    expect(() => run({ documentElement: root }, null)).not.toThrow();
    expect(root.dataset.os).toBeUndefined();
  });
});
