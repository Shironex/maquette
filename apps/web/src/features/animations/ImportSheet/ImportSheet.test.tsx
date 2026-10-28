import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import ImportSheet from './ImportSheet';

afterEach(() => {
  clearMocks();
  useAnimationsStore.setState({ view: null, importing: null });
});

async function open() {
  await openMockLibrary('demo');
  await useAnimationsStore.getState().load();
  useAnimationsStore.getState().openImport();
  render(<ImportSheet />);
}

describe('the import sheet', () => {
  it('reads the picked files and shows what each one adds', async () => {
    await open();
    expect(screen.getByRole('button', { name: 'Import' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Add files' }));
    const sheet = screen.getByRole('dialog', { name: 'Import clips' });
    expect(await within(sheet).findByText('Pro Sword and Shield Pack.zip')).toBeVisible();
    expect(sheet).toHaveTextContent(
      'Sword And Shield Idle is already in Pro Sword and Shield Pack'
    );
    expect(sheet).toHaveTextContent('magic_pose.fbx: it holds one pose and no keys');
    expect(sheet).toHaveTextContent('Rig: custom, 58 bones, not Mixamo');
    expect(within(sheet).getByLabelText('Pack for Pro Magic Pack.zip')).toHaveValue(
      'Pro Magic Pack'
    );
    expect(sheet).toHaveTextContent('Mixamo license.');
    expect(within(sheet).getByRole('button', { name: 'Import 8 clips' })).toBeVisible();
  });

  it('keeps a duplicate when asked and imports', async () => {
    await open();
    await userEvent.click(screen.getByRole('button', { name: 'Add files' }));
    const keep = await screen.findByRole('group', {
      name: 'Sword And Shield Idle: skip or keep both',
    });
    await userEvent.click(within(keep).getByRole('button', { name: 'Keep both' }));
    await userEvent.click(screen.getByRole('button', { name: 'Import 9 clips' }));
    expect(useAnimationsStore.getState().importing).toBeNull();
    const view = useAnimationsStore.getState().view!;
    expect(view.items.some(i => i.clip.name === 'Magic Area Heal')).toBe(true);
  });
});
