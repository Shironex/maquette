import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PromptField } from '.';

describe('the texture prompt field', () => {
  it('counts the characters and passes the text on', () => {
    const onChange = vi.fn();
    render(<PromptField value="Mossy stone" onChange={onChange} />);
    expect(screen.getByText('11 of 800 characters')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Texture prompt'), { target: { value: 'Clay' } });
    expect(onChange).toHaveBeenCalledWith('Clay');
  });

  it('warns when the text is longer than Meshy takes', () => {
    render(<PromptField value={'x'.repeat(801)} onChange={() => {}} name="Lamp" />);
    expect(screen.getByLabelText('Texture prompt for Lamp')).toBeInTheDocument();
    expect(
      screen.getByText(/801 of 800 characters\. Meshy refuses longer ones\./)
    ).toBeInTheDocument();
  });
});
