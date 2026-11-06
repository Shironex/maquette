/**
 * The toast an approval's answer shows, one case per kind. The switch is
 * exhaustive, so a new kind fails the typecheck here instead of borrowing
 * another kind's words (a denied recipe save once read "Run null stopped").
 */
import type { Approval } from '@/ipc/client';

export interface ApprovalToast {
  status: 'done' | 'info';
  title: string;
  detail: string;
}

const TOLD = 'The agent is told it was denied.';

const toast = (approve: boolean, title: string, detail: string): ApprovalToast => ({
  status: approve ? 'done' : 'info',
  title,
  detail,
});

/**
 * What answering `approval` with `approve` says. `game` is whether its
 * project ships into a game repo, which only a Ship cares about.
 */
export function approvalToast(approval: Approval, approve: boolean, game: boolean): ApprovalToast {
  switch (approval.kind) {
    case 'send-to-3d': {
      const asset = approval.assetName ?? 'The asset';
      return approve
        ? toast(
            true,
            `${asset} is on its way to ${approval.service ?? '3D'}`,
            'Follow it in Sends. The agent sees it on its next get_provider_job.'
          )
        : toast(false, `${asset} was not sent`, `Nothing was spent. ${TOLD}`);
    }
    case 'image-round':
      return approve
        ? toast(true, 'The images are on their way', 'Follow them in the Lab.')
        : toast(false, 'The images were not made', `Nothing was spent. ${TOLD}`);
    case 'page-preview':
      return approve
        ? toast(true, 'The dev server is starting', 'A changed command or config asks again.')
        : toast(false, 'The dev command did not run', `Nothing ran. ${TOLD}`);
    case 'recipe-save': {
      const recipe = approval.assetName ?? 'The recipe';
      return approve
        ? toast(true, `${recipe} was saved to the library`, 'The agent sees it in the library.')
        : toast(false, 'Not saved to the library', `The library did not change. ${TOLD}`);
    }
    case 'recipe-bake':
      return approve
        ? toast(true, `${approval.assetName ?? 'The bake'} was added`, 'It is the next version.')
        : toast(false, 'Not added', `The bake was not added as a version. ${TOLD}`);
    case 'rig-doctor':
      return approve
        ? toast(true, `${approval.assetName ?? 'The draft'} was added`, 'It is the next version.')
        : toast(false, 'Not added', `The draft was not added as a version. ${TOLD}`);
    case 'rest-pose':
    case 'reweight':
    case 'springs': {
      const name = approval.assetName ?? 'The avatar';
      return approve
        ? toast(true, `${name} was approved`, 'The agent sees it on its next call.')
        : toast(false, `${name} was turned down`, `Nothing was written. ${TOLD}`);
    }
    case 'ship': {
      const run = `Run ${approval.runNumber ?? '?'}`;
      if (approve) {
        return toast(
          true,
          `${run} is shipping`,
          game
            ? 'Ship never commits. Review the change in git and commit it yourself.'
            : 'The agent sees the result on its next get_run.'
        );
      }
      return toast(
        false,
        `${run} stopped`,
        game ? 'Ship did not run. Nothing in the game repo changed.' : `Ship did not run. ${TOLD}`
      );
    }
    default: {
      const unknown: never = approval.kind;
      return unknown;
    }
  }
}
