import { useEffect, useState } from 'react';
import { commands, errorMessage, type Approval } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useNow } from '@/lib/useNow';
import { commitFirstLine, originalsLine, shipGroups } from '@/state/game';
import { formatCredits } from '@/state/credits';
import { formatUsd } from '@/state/image-api';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/**
 * For a game's Ship: which replaced files are the game's imported originals,
 * and whether the game repo has changes to commit first. Empty otherwise.
 */
function useShipNotes(approval: Approval, game: boolean): string[] {
  const [notes, setNotes] = useState<string[]>([]);
  const { project, runId } = approval;
  useEffect(() => {
    if (!game) return;
    let live = true;
    commands
      .gameShipPlan(project)
      .then(plan => {
        const lines = [originalsLine(shipGroups(plan)), commitFirstLine(plan)];
        if (live) setNotes(lines.filter((l): l is string => Boolean(l)));
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [game, project, runId]);
  return game ? notes : [];
}

/** The project an approval is for, the clock for its age, and the decision. */
export function useApprovalCard(approval: Approval) {
  const decide = useAgentsStore(s => s.decide);
  const deciding = useAgentsStore(s => s.deciding === approval.id);
  const projects = useAppStore(s => s.workspace?.projects);
  const site = projects?.find(p => p.id === approval.project);
  const now = useNow(15_000);
  const estimate = approval.estimate;
  const usd = approval.estimateUsd;
  const cost =
    approval.kind === 'image-round'
      ? usd === null || usd === undefined
        ? 'dollars from the OpenAI key'
        : `about ${formatUsd(usd)} from the OpenAI key`
      : estimate === null || estimate === undefined
        ? `${approval.service} credits`
        : `about ${formatCredits(estimate)} ${approval.service} credits`;
  const notes = useShipNotes(approval, approval.kind === 'ship' && site?.target === 'game');
  return { decide, deciding, site, now, cost, notes, pose: useRestPose(approval) };
}

/**
 * A rest pose is judged on its sheets, so the card only opens them in the
 * Avatar view or turns the pose down; approving happens there.
 */
function useRestPose(approval: Approval) {
  const [denying, setDenying] = useState(false);
  const notify = useAppStore(s => s.notify);
  const { project, asset, version, sheets } = approval;
  return {
    sheets: sheets ? { before: fileUrl(sheets.before), after: fileUrl(sheets.after) } : null,
    denying,
    open: () => {
      if (asset) useAvatarStore.getState().open(project, asset, version ?? null);
    },
    deny: async () => {
      if (!asset) return;
      setDenying(true);
      try {
        await commands.avatarDecide(project, asset, version ?? null, false);
        notify({
          status: 'info',
          title: `${approval.assetName ?? 'The avatar'}'s rest pose was turned down`,
          detail: 'Nothing was written. The agent is told it was denied.',
        });
        await useAgentsStore.getState().load();
        useAvatarStore.getState().onChanged(project, asset);
      } catch (error) {
        notify({
          status: 'failed',
          title: 'Could not answer the rest pose',
          detail: errorMessage(error),
        });
      } finally {
        setDenying(false);
      }
    },
  };
}
