/**
 * Credits: what each provider says is left, what paid sends spent this
 * week and month, how much went on redos, and every spend of the
 * workspace's ledger, filterable and saved as CSV. API spend and By hand
 * subscription spend show one at a time, so their totals never mix.
 */
import { Button, Icon, IconButton, InlineBanner, SegmentedControl, Select } from '@maquette/ui';
import type { ProviderId, SpendSource } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { SOURCE_LABEL, formatCredits, shareText, spendKey } from '@/state/credits';
import { SectionTitle } from '../SectionTitle';
import { Sheet } from '../Sheet';
import { BalanceCard } from './BalanceCard';
import { useCreditsSheet } from './CreditsSheet.hooks';
import { SpendRow } from './SpendRow';

const SERVICES = [
  { value: 'all', label: 'All' },
  { value: 'tripo', label: 'Tripo' },
  { value: 'meshy', label: 'Meshy' },
] as const;

const SOURCES = (['api', 'subscription'] as const).map(value => ({
  value,
  label: SOURCE_LABEL[value],
}));

const EMPTY: Record<SpendSource, string> = {
  api: 'Sends to Tripo and Meshy land here with what they cost.',
  subscription: 'By hand sends land here with what the site charges your plan, about.',
};

export default function CreditsSheet() {
  const {
    open,
    close,
    ledger,
    balances,
    checking,
    checkBalances,
    error,
    loading,
    filter,
    setFilter,
    exportCsv,
    now,
    spends,
    totals,
    nameOf,
    projectOptions,
    showProject,
  } = useCreditsSheet();
  if (!open) return null;
  const checkedAt = balances?.[0]?.checkedAtMs;
  const cards = (['tripo', 'meshy'] as const).map(service => (
    <BalanceCard
      key={service}
      service={service}
      balance={balances?.find(b => b.service === service)}
      checking={checking}
    />
  ));
  const rows = spends.map(spend => (
    <SpendRow
      key={spendKey(spend)}
      spend={spend}
      now={now}
      project={showProject ? nameOf(spend.project) : null}
    />
  ));
  const stats = [
    { label: 'Last 7 days', value: formatCredits(totals.week), sub: 'credits' },
    { label: 'Last 30 days', value: formatCredits(totals.month), sub: 'credits' },
    {
      label: 'Redo share',
      value: shareText(totals.redoShare),
      sub: `${totals.redos} of ${totals.sends} sends`,
    },
  ].map(s => (
    <div
      key={s.label}
      className="flex flex-col gap-0.5 rounded-card border border-line bg-raised px-3 py-2.5"
    >
      <span className="text-caption text-fg-mute uppercase">{s.label}</span>
      <span className="text-[20px] leading-7 font-semibold tabular-nums">{s.value}</span>
      <span className="text-meta text-fg-mute">{s.sub}</span>
    </div>
  ));

  return (
    <Sheet
      id="credits"
      icon="gauge"
      title="Credits"
      meta="Paid sends of this workspace, kept in credits.jsonl"
      onClose={close}
      width={720}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <section className="flex flex-col gap-2.5">
          <SectionTitle
            aside={
              <span className="inline-flex items-center gap-1.5">
                {checkedAt ? `Checked ${formatAgo(checkedAt, now)}` : null}
                <IconButton
                  icon="arrow-clockwise"
                  size="sm"
                  label="Ask for the balances again"
                  disabled={checking}
                  onClick={() => void checkBalances()}
                />
              </span>
            }
          >
            API balance
          </SectionTitle>
          <div className="grid grid-cols-2 gap-2.5">{cards}</div>
        </section>

        <section className="flex min-h-0 flex-col gap-2.5">
          <SectionTitle>Spent</SectionTitle>
          <div className="flex flex-wrap items-end gap-3">
            <SegmentedControl
              label="Paid with"
              options={SOURCES}
              value={filter.source}
              onChange={v => setFilter({ source: v as SpendSource })}
            />
            <Select
              label="Project"
              size="sm"
              options={projectOptions}
              value={filter.project ?? ''}
              onChange={project => setFilter({ project: project || null })}
              fieldClassName="w-[200px]"
            />
            <SegmentedControl
              label="Provider"
              options={SERVICES}
              value={filter.service ?? 'all'}
              onChange={v => setFilter({ service: v === 'all' ? null : (v as ProviderId) })}
            />
          </div>
          <div className="grid grid-cols-3 gap-2.5">{stats}</div>
          {error ? (
            <InlineBanner tone="danger" title="Could not read the ledger" detail={error} />
          ) : loading && !ledger ? (
            <p className="flex items-center gap-2 py-4 text-fg-mute">
              <Icon name="circle-notch" spin size={16} /> Reading the ledger
            </p>
          ) : spends.length === 0 ? (
            <p className="rounded-card border border-dashed border-line-strong px-3 py-6 text-center text-meta text-fg-mute">
              No {filter.source === 'api' ? 'paid' : 'subscription'} sends
              {filter.project || filter.service ? ' for this filter' : ' yet'}.{' '}
              {EMPTY[filter.source]}
            </p>
          ) : (
            <div className="rounded-card border border-line">
              <div className="grid grid-cols-[92px_minmax(0,1fr)_minmax(0,1.2fr)_96px_84px] gap-3 border-b border-line px-3 py-2 text-caption text-fg-mute uppercase">
                <span>Sent</span>
                <span>Asset</span>
                <span>Settings</span>
                <span>Outcome</span>
                <span className="text-right">Credits</span>
              </div>
              <ul className="m-0 list-none p-0">{rows}</ul>
            </div>
          )}
          {filter.source === 'subscription' ? (
            <p className="text-meta text-fg-mute">
              Subscription credits are estimates from the table in Maquette, paid from your plan on
              the Tripo or Meshy site, never from the API balance above.
            </p>
          ) : totals.estimated > 0 ? (
            <p className="text-meta text-fg-mute">
              <span className="font-medium text-fg-dim">est.</span> marks credits from the price
              list, for sends still running or that did not report a cost.
            </p>
          ) : null}
        </section>
      </div>
      <footer className="flex items-center gap-2 border-t border-line px-5 py-3">
        <span className="mr-auto text-meta text-fg-mute">
          {ledger?.skipped
            ? `${ledger.skipped} ${ledger.skipped === 1 ? 'line' : 'lines'} of the ledger could not be read and ${ledger.skipped === 1 ? 'is' : 'are'} left out.`
            : `${spends.length} ${spends.length === 1 ? 'send' : 'sends'}`}
        </span>
        <Button
          icon="download-simple"
          disabled={spends.length === 0}
          disabledReason="Nothing to save for this filter"
          onClick={() => void exportCsv()}
        >
          Export CSV
        </Button>
        <Button variant="primary" onClick={close}>
          Done
        </Button>
      </footer>
    </Sheet>
  );
}
