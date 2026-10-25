/**
 * Settings > 3D providers: the Tripo and Meshy API keys. They live in the
 * OS keychain; the UI only ever sees the last four characters.
 */
import { KeyRow } from './KeyRow';
import { KEYCHAIN } from './keychain';

const SERVICES = ['tripo', 'meshy'] as const;

export default function KeyFields() {
  const rows = SERVICES.map(service => <KeyRow key={service} service={service} />);
  return (
    <div className="flex flex-col gap-2">
      {rows}
      <p className="text-meta text-fg-mute">
        Keys stay in the {KEYCHAIN}, never in maquette.json, a project file or a log. Test key uses
        the free balance call, so it spends nothing.
      </p>
    </div>
  );
}
