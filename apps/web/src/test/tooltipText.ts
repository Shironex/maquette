/**
 * Reads a control's tooltip the way a person does: hover, wait out the
 * delay, read the bubble, then leave so the next one starts cold.
 */
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

export async function tooltipText(target: HTMLElement): Promise<string> {
  await userEvent.hover(target);
  const tip = await screen.findByRole('tooltip');
  const text = tip.textContent ?? '';
  await userEvent.unhover(target);
  return text;
}
