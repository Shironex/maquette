export interface SelectionBarProps {
  count: number;
  /** "all checks pass", "2 to review". */
  note: string;
  /** How many of the selection Accept takes (unreadable files stay). */
  acceptable: number;
  /** Why the selection cannot be redone together, or null. */
  redoBlocker: string | null;
  /** A bulk accept or discard is running. */
  working?: boolean;
  /** `wide` lays every action in one row and offers Accept and add to library. */
  wide?: boolean;
  onAccept: () => void;
  onAcceptToLibrary?: () => void;
  onRedo: () => void;
  onDiscard: () => void;
  onClear: () => void;
}
