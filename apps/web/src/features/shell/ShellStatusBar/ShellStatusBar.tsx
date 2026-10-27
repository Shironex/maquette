import { StatusBar, type StatusDot } from '@maquette/ui';
import { SiteServerItem } from '../SiteServerItem';
import { useShellStatusBar } from './ShellStatusBar.hooks';

const siteControl = (label: string, dot: StatusDot) => <SiteServerItem label={label} dot={dot} />;

export default function ShellStatusBar() {
  const items = useShellStatusBar(siteControl);
  return <StatusBar items={items} />;
}
