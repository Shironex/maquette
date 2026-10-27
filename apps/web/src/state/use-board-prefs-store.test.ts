import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_PREFS, prefsKey } from './board-list';
import { forgetStoredPrefs, prefsOf, useBoardPrefsStore } from './use-board-prefs-store';

afterEach(() => {
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
  vi.restoreAllMocks();
});

describe('the Board prefs', () => {
  it('remembers the layout, sort and filter per project', () => {
    useBoardPrefsStore.getState().update('frostveil', { layout: 'list', stage: 'web' });
    expect(JSON.parse(window.localStorage.getItem(prefsKey('frostveil'))!)).toMatchObject({
      layout: 'list',
      stage: 'web',
    });
    expect(prefsOf(useBoardPrefsStore.getState().byProject, 'frost-cathedral')).toEqual(
      DEFAULT_PREFS
    );
    // A fresh start reads what was stored.
    useBoardPrefsStore.setState({ byProject: {} });
    forgetStoredPrefs();
    const back = prefsOf({}, 'frostveil');
    expect(back).toMatchObject({ layout: 'list', stage: 'web' });
    // The same object every time, so a selector stays stable.
    expect(prefsOf({}, 'frostveil')).toBe(back);
  });

  it('starts from the defaults when storage throws', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(prefsOf({}, 'frostveil')).toEqual(DEFAULT_PREFS);
    useBoardPrefsStore.getState().update('frostveil', { layout: 'list' });
    expect(prefsOf(useBoardPrefsStore.getState().byProject, 'frostveil').layout).toBe('list');
  });
});
