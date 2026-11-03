import { describe, expect, it } from 'vitest';
import type { Asset, ProjectManifest, ProviderJob } from '../client';
import { deniedCardBack } from './denied-card-back';

const asset = (over: Partial<Asset>): Asset =>
  ({ id: 'lamp', stage: 'tripo', status: 'queued', versions: [], ...over }) as Asset;
const project = (a: Asset): ProjectManifest => ({ id: 'moon', assets: [a] }) as ProjectManifest;
const job = (over: Partial<ProviderJob>): ProviderJob =>
  ({ id: 'lamp-1', project: 'moon', asset: 'lamp', state: 'cancelled', ...over }) as ProviderJob;

describe('deniedCardBack', () => {
  it('puts a card with no model back in Refs', () => {
    const next = deniedCardBack(project(asset({})), job({}), [job({})]);
    expect(next.assets?.[0]?.stage).toBe('refs');
  });

  it('leaves a card with a model or another send where it is', () => {
    const modelled = asset({ versions: [{ version: 1 }] as Asset['versions'] });
    expect(deniedCardBack(project(modelled), job({}), []).assets?.[0]?.stage).toBe('tripo');
    const other = job({ id: 'lamp-2', state: 'waiting' });
    expect(deniedCardBack(project(asset({})), job({}), [other]).assets?.[0]?.stage).toBe('tripo');
  });
});
