import { StatusBar } from '@maquette/ui';
import { useShellStatusBar } from './ShellStatusBar.hooks';

export default function ShellStatusBar() {
  const items = useShellStatusBar();
  return <StatusBar items={items} />;
}
