import type { ProviderId } from '@/ipc/client';
import { SERVICE_LABEL } from '@/state/providers';
import { KeyRowView } from '../KeyRowView';
import { useKeyRow } from './KeyRow.hooks';

const WHERE: Record<ProviderId, string> = {
  tripo: 'platform.tripo3d.ai, API keys',
  meshy: 'meshy.ai, Settings, API',
};

export interface KeyRowProps {
  service: ProviderId;
}

/** A 3D provider's key. */
export default function KeyRow({ service }: KeyRowProps) {
  return (
    <KeyRowView
      label={SERVICE_LABEL[service]}
      where={WHERE[service]}
      testTip="Checks the saved key with the provider"
      {...useKeyRow(service)}
    />
  );
}
