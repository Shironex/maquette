/**
 * The test DOM neither lays out nor scrolls. This gives every element a tall
 * scroll height and a `scrollTo` that moves `scrollTop` and fires `scroll`,
 * as a browser does, so windowed lists can be scrolled by code under test.
 * Returns the undo.
 */
export function fakeScroll(): () => void {
  const proto = HTMLElement.prototype;
  Object.defineProperty(proto, 'scrollHeight', { get: () => 1e6, configurable: true });
  Object.defineProperty(proto, 'scrollTo', {
    configurable: true,
    value(this: HTMLElement, to: ScrollToOptions) {
      this.scrollTop = to.top ?? this.scrollTop;
      this.dispatchEvent(new Event('scroll'));
    },
  });
  return () => {
    Reflect.deleteProperty(proto, 'scrollHeight');
    Reflect.deleteProperty(proto, 'scrollTo');
  };
}
