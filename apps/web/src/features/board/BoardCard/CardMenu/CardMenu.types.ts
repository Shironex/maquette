import type { Asset, BuiltAsset, ProviderJob } from '@/ipc/client';
import type { BoardEnv } from '../../board.types';

export interface CardMenuProps {
  asset: Asset;
  /** The card's By hand job while it waits for a model. */
  waiting: ProviderJob | undefined;
  /** The card's picture when it is a ref, for Make object refs. */
  refPicture: string | null;
  /** How the card stands against its script, when a script builds it. */
  built?: BuiltAsset;
  board: BoardEnv;
}
