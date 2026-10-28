import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
  type MouseEvent,
} from 'react';
import { useModalFocus } from '@/lib/useModalFocus';
import { useAppStore } from '@/state/use-app-store';
import { emptyProjectFields, projectFieldsReady, type ProjectFields } from '@/state/onboarding';
import { useProjectsStore } from '@/state/use-projects-store';
import { useProjectFieldActions } from '../../useProjectFieldActions';

/** The form state, Escape to close, and submit on Enter or Cmd+Enter. */
export function useNewProjectSheet(initialMode: ProjectFields['mode'], onClosed?: () => void) {
  const close = useProjectsStore(s => s.closeDialog);
  const create = useProjectsStore(s => s.create);
  const creating = useProjectsStore(s => s.creating);
  const current = useAppStore(s => s.workspace?.project?.name);
  const [fields, setFields] = useState<ProjectFields>(() => ({
    ...emptyProjectFields(),
    mode: initialMode,
  }));
  const update = (patch: Partial<ProjectFields>) => setFields(f => ({ ...f, ...patch }));
  const actions = useProjectFieldActions(fields, update);
  const sheet = useRef<HTMLFormElement>(null);
  useModalFocus(sheet);
  const ready = projectFieldsReady(fields);
  const game = fields.target === 'game';
  const importing = !game && fields.mode === 'import';

  const dismiss = () => {
    close();
    onClosed?.();
  };

  useEffect(() => {
    if (initialMode === 'import') sheet.current?.querySelector<HTMLElement>('input')?.focus();
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      close();
      onClosed?.();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [close, onClosed, initialMode]);

  const submit = async () => {
    if (!ready) return;
    if (await create(fields)) onClosed?.();
  };

  const onBackdropClick = (e: MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) dismiss();
  };

  const onSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    void submit();
  };

  const onKeyDown = (e: KeyboardEvent<HTMLFormElement>) => {
    if (e.key !== 'Enter' || !(e.metaKey || e.ctrlKey)) return;
    e.preventDefault();
    void submit();
  };

  return {
    fields,
    update,
    actions,
    sheet,
    ready,
    importing,
    game,
    creating,
    current,
    dismiss,
    onBackdropClick,
    onSubmit,
    onKeyDown,
  };
}
