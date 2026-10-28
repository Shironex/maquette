/**
 * A grid card whose one control is a button laid over all of it: the card
 * holds the picture and the lines of text, the button only the name, so a
 * screen reader hears what the card is called and reads the rest as the
 * text around it.
 */
export const CARD =
  'relative w-full rounded-card border border-line bg-raised text-left transition-colors duration-120 hover:border-line-strong';

/** The button over the card; its focus ring sits inside the card's edge, which clips. */
export const CARD_BUTTON =
  'absolute inset-0 z-10 cursor-pointer rounded-card focus-visible:outline-offset-[-2px]';
