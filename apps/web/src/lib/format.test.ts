import { describe, expect, it } from 'vitest';
import {
  formatAgo,
  formatBytes,
  formatClock,
  formatCount,
  formatK,
  homePath,
  shortPath,
} from './format';

describe('format', () => {
  it('formats sizes with decimal units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(840_000)).toBe('840 KB');
    expect(formatBytes(2_300_000_000)).toBe('2.3 GB');
    expect(formatBytes(5_100_000)).toBe('5.1 MB');
  });

  it('formats a time of day as HH:MM in local time', () => {
    expect(formatClock(new Date(2026, 9, 26, 9, 5, 40).getTime())).toBe('09:05');
    expect(formatClock(new Date(2026, 9, 26, 23, 59).getTime())).toBe('23:59');
  });

  it('formats elapsed time the way the status bar reads', () => {
    expect(formatAgo(0, 1_000)).toBe('just now');
    expect(formatAgo(0, 4_000)).toBe('4 s ago');
    expect(formatAgo(0, 120_000)).toBe('2 min ago');
  });

  it('formats counts', () => {
    expect(formatCount(60000)).toBe('60,000');
    expect(formatK(50000)).toBe('50k');
    expect(formatK(18200)).toBe('18.2k');
  });

  it('shortens long paths to their tail', () => {
    expect(shortPath('~/Documents/Projects/site/apps/landing')).toBe('site/apps/landing');
    expect(shortPath('C:\\Users\\me\\site', 2)).toBe('me/site');
  });
});

describe('homePath', () => {
  it('shows the home folder as a tilde on both separators', () => {
    expect(homePath('/Users/maker/Maquette', '/Users/maker')).toBe('~/Maquette');
    expect(homePath('C:\\Users\\maker\\Maquette', 'C:\\Users\\maker')).toBe('~\\Maquette');
    expect(homePath('/Users/makers/x', '/Users/maker')).toBe('/Users/makers/x');
    expect(homePath('/opt/x', null)).toBe('/opt/x');
  });
});

describe('formatAgo in days', () => {
  it('switches to days after two days', () => {
    expect(formatAgo(0, 47 * 3_600_000)).toBe('47 h ago');
    expect(formatAgo(0, 144 * 3_600_000)).toBe('6 days ago');
  });
});
