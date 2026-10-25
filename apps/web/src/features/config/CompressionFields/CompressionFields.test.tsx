import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { CompressionFields } from '.';

describe('the compression fields', () => {
  it('patch the texture codec', async () => {
    const config = { ...defaultConfig() };
    config.compression = { ...config.compression, textureCodec: 'etc1s' };
    const onChange = vi.fn();
    render(<CompressionFields config={config} onChange={onChange} />);
    await userEvent.click(screen.getByRole('button', { name: 'UASTC' }));
    expect(onChange).toHaveBeenCalledWith({
      ...config,
      compression: { ...config.compression, textureCodec: 'uastc' },
    });
  });
});
