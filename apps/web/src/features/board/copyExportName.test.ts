import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { copyExportName } from './copyExportName';

afterEach(() => useAppStore.setState({ notice: null }));

describe('copying an export name', () => {
  it('puts the name on the clipboard and says which', async () => {
    // Installs the clipboard stub jsdom lacks.
    userEvent.setup();
    await copyExportName('maq_iron-street-lamp_v1_tripo');
    expect(await navigator.clipboard.readText()).toBe('maq_iron-street-lamp_v1_tripo');
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'done',
      title: 'Export name copied',
      detail: 'maq_iron-street-lamp_v1_tripo',
    });
  });
});
