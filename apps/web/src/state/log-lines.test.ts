import { describe, expect, it } from 'vitest';
import { classifyLog, sizeChange } from './log-lines';

const kind = (text: string, level?: 'info' | 'ok' | 'warn' | 'error') =>
  classifyLog(text, level).kind;

describe('classifyLog', () => {
  it('flags errors', () => {
    expect(kind('Traceback (most recent call last):')).toBe('error');
    expect(kind('KeyError: \'bpy_prop_collection[key]: key "poster" not found\'')).toBe('error');
    expect(kind('Capture posters failed after 3.2s: blender exited with code 1.')).toBe('error');
    expect(kind('Run 3 failed: Capture posters failed.')).toBe('error');
    expect(kind('anything the runner marked', 'error')).toBe('error');
  });

  it('flags warnings', () => {
    expect(kind('| WARNING: More than one shader node tex image used for a texture.')).toBe(
      'warning'
    );
    expect(kind('Warning: texture is not a power of two')).toBe('warning');
    expect(kind('quiet but marked', 'warn')).toBe('warning');
  });

  it('marks step milestones and approvals', () => {
    expect(kind('Build scene done in 1.9s', 'ok')).toBe('milestone');
    expect(kind('Run 3 done in 56.1s, 0 cached', 'ok')).toBe('milestone');
    expect(kind('Ship approved')).toBe('milestone');
    expect(kind('Build scene inputs unchanged, cache hit', 'ok')).toBe('milestone');
    expect(kind('Ship waits for approval in Maquette, because an agent started this run.')).toBe(
      'waiting'
    );
  });

  it('splits our script tag from the rest', () => {
    expect(classifyLog('[build_scene] imported book_stack_v1.glb: 1 objects')).toEqual({
      kind: 'tagged',
      tag: 'build_scene',
      body: 'imported book_stack_v1.glb: 1 objects',
    });
    expect(classifyLog('[bake_light] baking 4 maps').tag).toBe('bake_light');
    expect(kind('[ship] copied 3 files')).toBe('tagged');
    expect(kind('[not a tag] words')).toBe('plain');
  });

  it('finds results', () => {
    expect(kind('info: book_stack_v1.glb (1.27 MB) → book_stack_v1.tex.glb (653.13 KB)')).toBe(
      'result'
    );
    expect(kind('wrote web/raw/veil_platform.glb 4.6 MB')).toBe('result');
    expect(kind('Saved web/posters/hero.webp')).toBe('result');
  });

  it('dims the tools chatter', () => {
    for (const text of [
      '00:15:49 00:15:49 | INFO: Blender create Mesh node ...',
      '15:02:22 | INFO: Starting glTF 2.0 export',
      'info: prune: Removed types... Accessor (5)',
      'Blender quit',
      'Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17)',
      'Info: Baking map saved to internal image, save it externally or pack it',
      'Draco compression available',
      'MeshOptimizer not available',
    ]) {
      expect(kind(text), text).toBe('noise');
    }
  });

  it('leaves other lines plain', () => {
    expect(kind('Read blend: "blender/scene_baked.blend"')).toBe('plain');
    expect(kind('veil_platform geometry with meshopt high', 'info')).toBe('plain');
  });
});

describe('sizeChange', () => {
  it('marks the size the arrow ends at', () => {
    expect(sizeChange('info: a.glb (1.27 MB) → b.tex.glb (653.13 KB)')).toEqual({
      before: 'info: a.glb (1.27 MB) →',
      between: ' b.tex.glb (',
      size: '653.13 KB',
      after: ')',
    });
    expect(sizeChange('4.6 MB → 2.1 MB')?.size).toBe('2.1 MB');
  });

  it('gives nothing without an arrow or a size', () => {
    expect(sizeChange('wrote web/raw/veil_platform.glb 4.6 MB')).toBeNull();
    expect(sizeChange('a → b')).toBeNull();
  });
});
