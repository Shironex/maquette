import { describe, expect, it } from 'vitest';
import { cn } from '../lib/cn';

describe('cn', () => {
  it('keeps a type size and a colour that share the text- prefix', () => {
    expect(cn('text-body text-fg')).toBe('text-body text-fg');
  });

  it('lets a later colour win over an earlier one', () => {
    expect(cn('text-fg-dim', 'text-fg')).toBe('text-fg');
    expect(cn('bg-raised', 'bg-hover')).toBe('bg-hover');
  });

  it('lets a later kit radius win', () => {
    expect(cn('rounded-control', 'rounded-card')).toBe('rounded-card');
  });
});
