import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import {
  AssetCard,
  CheckRow,
  HelpMark,
  Tip,
  LogConsole,
  NavRail,
  ProjectMenu,
  PipelineNode,
  budgetScale,
  NumberField,
  SegmentedControl,
  StageStepper,
  TextArea,
  Toggle,
  ToolCheckRow,
} from '../index';

describe('budgetScale', () => {
  it('parks the target at 80 percent while under budget', () => {
    expect(budgetScale(1.5, 3).targetAt).toBeCloseTo(80);
    expect(budgetScale(1.5, 3).over).toBe(false);
  });

  it('fills the whole bar once over budget', () => {
    const s = budgetScale(5.1, 3);
    expect(s.over).toBe(true);
    expect(s.fill).toBe(100);
  });

  it('copes with an empty project', () => {
    expect(budgetScale(0, 3).fill).toBe(0);
  });
});

describe('NumberField', () => {
  it('commits a clamped number on blur', async () => {
    const onChange = vi.fn();
    render(<NumberField label="Tris per model" value={60000} onChange={onChange} min={1000} />);
    const input = screen.getByLabelText('Tris per model');
    await userEvent.clear(input);
    await userEvent.type(input, '12');
    await userEvent.tab();
    expect(onChange).toHaveBeenCalledWith(1000);
  });

  it('reads thousands separators and reverts text it cannot read', async () => {
    const onChange = vi.fn();
    render(<NumberField label="Quads" value={50000} onChange={onChange} />);
    const input = screen.getByLabelText('Quads');
    await userEvent.clear(input);
    await userEvent.type(input, '42,000{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(42000);
    await userEvent.clear(input);
    await userEvent.type(input, 'lots');
    await userEvent.tab();
    expect(input).toHaveValue('50000');
  });
});

describe('SegmentedControl', () => {
  it('marks the chosen option pressed', async () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        label="Texture codec"
        options={[
          { value: 'etc1s', label: 'ETC1S' },
          { value: 'uastc', label: 'UASTC' },
        ]}
        value="etc1s"
        onChange={onChange}
      />
    );
    expect(screen.getByRole('button', { name: 'ETC1S' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'UASTC' }));
    expect(onChange).toHaveBeenCalledWith('uastc');
  });

  it('shows a disabled option without choosing it', async () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        label="Units"
        options={[
          { value: 'percent', label: 'Scroll %' },
          { value: 'pixels', label: 'Pixels', disabled: true, hint: 'Once the page answers' },
        ]}
        value="percent"
        onChange={onChange}
      />
    );
    const pixels = screen.getByRole('button', { name: 'Pixels' });
    expect(pixels).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(pixels);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe('Toggle', () => {
  it('is a switch with an accessible name', async () => {
    const onChange = vi.fn();
    render(<Toggle aria-label="Ask before shipping" checked onChange={onChange} />);
    const sw = screen.getByRole('switch', { name: 'Ask before shipping' });
    expect(sw).toBeChecked();
    await userEvent.click(sw);
    expect(onChange).toHaveBeenCalledWith(false);
  });
});

describe('CheckRow', () => {
  it('shows a footer under the detail, in the text column', () => {
    render(
      <CheckRow
        status="warning"
        title="94.6k triangles"
        detail="Over the budget."
        footer={<button type="button">Decimate to 60k</button>}
      />
    );
    const fix = screen.getByRole('button', { name: 'Decimate to 60k' });
    expect(screen.getByText('Over the budget.').parentElement).toContainElement(fix);
  });
});

describe('ToolCheckRow', () => {
  it('reads a missing required tool as failed and an optional one as a warning', () => {
    const { rerender } = render(<ToolCheckRow name="Blender" required status="missing" />);
    expect(screen.getByRole('img', { name: 'Missing' })).toHaveClass('text-danger');
    rerender(<ToolCheckRow name="ffmpeg" status="missing" />);
    expect(screen.getByRole('img', { name: 'Missing' })).toHaveClass('text-amber');
  });
});

describe('StageStepper', () => {
  it('says each stage state in words, not only colour', () => {
    render(
      <StageStepper
        stages={[
          { id: 'concept', label: 'Concept', state: 'done' },
          { id: 'refs', label: 'Refs', state: 'current' },
        ]}
      />
    );
    expect(screen.getByText(', done')).toBeInTheDocument();
    expect(screen.getByText('Refs').closest('li')).toHaveAttribute('aria-current', 'step');
  });
});

describe('AssetCard', () => {
  it('opens its menu from the dots button', async () => {
    const onMore = vi.fn();
    const { rerender } = render(
      <AssetCard
        name="Island"
        version={7}
        status={{ state: 'done' }}
        more={{ onOpen: onMore, menu: <div role="menu">Move</div> }}
      />
    );
    const dots = screen.getByRole('button', { name: 'More for Island' });
    expect(dots).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(dots);
    expect(onMore).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
    rerender(
      <AssetCard
        name="Island"
        version={7}
        status={{ state: 'done' }}
        more={{ onOpen: onMore, open: true, menu: <div role="menu">Move</div> }}
      />
    );
    expect(screen.getByRole('menu')).toHaveTextContent('Move');
  });

  it('has a select box only when asked, and tells a shift click apart', async () => {
    const onCheck = vi.fn();
    const { rerender } = render(<AssetCard name="Desk lamp" />);
    expect(screen.queryByRole('checkbox')).toBeNull();
    rerender(
      <AssetCard
        name="Desk lamp"
        selection={{ checked: false, onCheck }}
        actions={<button type="button">Add model...</button>}
      />
    );
    const box = screen.getByRole('checkbox', { name: 'Select Desk lamp' });
    expect(box).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(box);
    expect(onCheck).toHaveBeenLastCalledWith(true, false);
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(box);
    await user.keyboard('{/Shift}');
    expect(onCheck).toHaveBeenLastCalledWith(true, true);
    expect(screen.getByRole('button', { name: 'Add model...' })).toBeInTheDocument();
  });
  it('shows a long status whole in a tooltip, and a note under it', async () => {
    const warning = 'Over 20k triangles, Texture over 1024 px';
    render(
      <AssetCard
        name="Crate"
        status={{
          state: 'review',
          label: warning,
          tooltip: warning,
          note: <code>maq_crate_v1_tripo</code>,
        }}
      />
    );
    const chip = screen.getByText(warning);
    expect(chip).toHaveClass('truncate');
    await userEvent.hover(chip);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(warning);
    expect(screen.getByText('maq_crate_v1_tripo')).toBeInTheDocument();
  });
});

describe('PipelineNode', () => {
  it('shows a custom chip label and actions', () => {
    render(
      <PipelineNode
        step={5}
        name="Capture posters"
        state="waiting"
        label="failed in 24"
        actions={<button type="button">Run from here</button>}
      />
    );
    expect(screen.getByText('failed in 24')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Run from here' })).toBeInTheDocument();
  });

  it('draws a known fraction while running, and an indeterminate bar otherwise', () => {
    const { rerender } = render(
      <PipelineNode step={2} name="Bake light" state="running" progress={68} />
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '68');
    rerender(<PipelineNode step={2} name="Bake light" state="running" />);
    const bar = screen.getByRole('progressbar');
    expect(bar).not.toHaveAttribute('aria-valuenow');
    expect(bar).toHaveAttribute('aria-busy', 'true');
    rerender(<PipelineNode step={2} name="Bake light" state="done" />);
    expect(screen.queryByRole('progressbar')).toBeNull();
  });
});

describe('LogConsole', () => {
  it('shows the empty text and keeps the newest line in view', () => {
    const { rerender } = render(
      <LogConsole title="Log" lines={[]} empty="No output yet" autoScroll />
    );
    expect(screen.getByRole('log')).toHaveTextContent('No output yet');
    const log = screen.getByRole('log');
    Object.defineProperty(log, 'scrollHeight', { value: 900, configurable: true });
    rerender(
      <LogConsole
        title="Log"
        lines={[{ t: '15:02:31', text: 'Blender quit' }]}
        empty="No output yet"
        autoScroll
      />
    );
    expect(log.scrollTop).toBe(900);
    expect(log).not.toHaveTextContent('No output yet');
  });
});

describe('TextArea', () => {
  it('labels the field and passes typing through', async () => {
    const onChange = vi.fn();
    render(<TextArea label="Prompt" hint="What the concepts show" onChange={onChange} />);
    const box = screen.getByLabelText('Prompt');
    expect(box.tagName).toBe('TEXTAREA');
    await userEvent.type(box, 'moon');
    expect(onChange).toHaveBeenCalledTimes(4);
  });
});

describe('ProjectMenu', () => {
  const three = [
    { id: 'moon-island', name: 'Moon Island', meta: 'Floating diorama', current: true },
    { id: 'rooftop-cinema', name: 'Rooftop cinema', meta: 'Room', live: 'Run 4 going' },
    { id: 'lantern-keeper', name: 'Lantern keeper', meta: 'Character showcase' },
  ];
  const actions = (onNew = vi.fn()) => [
    { id: 'new', label: 'New project', icon: 'plus' as const, onSelect: onNew },
    { id: 'import', label: 'Import folder', icon: 'folder-open' as const, onSelect: vi.fn() },
  ];

  it('checks the open project, focuses it and walks the rows with the arrow keys', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    const onNew = vi.fn();
    render(
      <ProjectMenu
        projects={three}
        actions={actions(onNew)}
        onSelect={onSelect}
        onClose={onClose}
        note="Run 25 keeps going in Moon Island."
      />
    );
    const moon = screen.getByRole('menuitemradio', { name: /Moon Island/ });
    expect(moon).toHaveAttribute('aria-checked', 'true');
    expect(moon).toHaveFocus();
    expect(screen.queryByRole('searchbox')).toBeNull();
    expect(screen.getByText('Run 4 going')).toBeInTheDocument();
    expect(screen.getByText('Run 25 keeps going in Moon Island.')).toBeInTheDocument();

    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('menuitemradio', { name: /Rooftop cinema/ })).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    expect(onSelect).toHaveBeenCalledWith('rooftop-cinema');

    await userEvent.keyboard('{End}');
    expect(screen.getByRole('menuitem', { name: 'Import folder' })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(moon).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}{ArrowUp}{Enter}');
    expect(onNew).toHaveBeenCalledTimes(1);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('searches once the list is long, from the field or by typing on a row', async () => {
    const many = Array.from({ length: 8 }, (_, i) => ({
      id: `p${i}`,
      name: i === 5 ? 'Harbour lights' : `Project ${i}`,
    }));
    const onSelect = vi.fn();
    render(
      <ProjectMenu projects={many} actions={actions()} onSelect={onSelect} onClose={vi.fn()} />
    );
    const search = screen.getByRole('searchbox', { name: 'Find a project' });
    expect(search).toHaveFocus();
    await userEvent.keyboard('harb');
    expect(screen.getAllByRole('menuitemradio')).toHaveLength(1);
    await userEvent.keyboard('{Enter}');
    expect(onSelect).toHaveBeenCalledWith('p5');
    await userEvent.clear(search);
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('menuitemradio', { name: 'Project 0' })).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}');
    expect(search).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}zz');
    expect(search).toHaveValue('zz');
    expect(screen.getByText('No project matches "zz".')).toBeInTheDocument();
  });

  it('closes on a click outside but not on its anchor', async () => {
    const onClose = vi.fn();
    const anchor = { current: null as HTMLButtonElement | null };
    render(
      <div>
        <button ref={el => void (anchor.current = el)}>Switcher</button>
        <button>Elsewhere</button>
        <ProjectMenu
          projects={three}
          actions={actions()}
          onSelect={vi.fn()}
          onClose={onClose}
          anchorRef={anchor}
        />
      </div>
    );
    await userEvent.click(screen.getByRole('button', { name: 'Switcher' }));
    expect(onClose).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Elsewhere' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe('NavRail', () => {
  it('keeps every tool reachable by name when collapsed', async () => {
    const onToggle = vi.fn();
    render(
      <NavRail
        items={[
          { id: 'board', label: 'Board', icon: 'squares-four' },
          { id: 'inbox', label: 'Inbox', icon: 'tray', badge: 3 },
        ]}
        footer={{ id: 'settings', label: 'Settings', icon: 'gear-six' }}
        value="board"
        onChange={vi.fn()}
        project={{ name: 'Moon Island', meta: 'Floating diorama', expanded: false }}
        collapsed
        onToggleCollapsed={onToggle}
      />
    );
    expect(screen.getByRole('button', { name: 'Inbox, 3 new' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Board' })).toHaveAttribute('aria-current', 'page');
    const switcher = screen.getByRole('button', { name: 'Project: Moon Island' });
    expect(switcher).toHaveAttribute('aria-haspopup', 'menu');
    expect(screen.queryByText('Tools')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Expand the sidebar' }));
    expect(onToggle).toHaveBeenCalled();
  });

  it('lists groups under their headings with plain counts', async () => {
    const onChange = vi.fn();
    render(
      <NavRail
        items={[{ id: 'board', label: 'Board', icon: 'squares-four' }]}
        groups={[
          {
            title: 'Libraries',
            items: [
              { id: 'library', label: 'Models', icon: 'cube', count: 38 },
              { id: 'animations', label: 'Animations', icon: 'person-simple', count: 132 },
            ],
          },
        ]}
        footer={{ id: 'settings', label: 'Settings', icon: 'gear-six' }}
        value="animations"
        onChange={onChange}
      />
    );
    const group = screen.getByRole('group', { name: 'Libraries' });
    expect(group).toHaveTextContent('Libraries');
    expect(group).toHaveTextContent('Models38');
    const animations = screen.getByRole('button', { name: /Animations/ });
    expect(animations).toHaveAttribute('aria-current', 'page');
    expect(animations).toHaveTextContent('132');
    await userEvent.click(screen.getByRole('button', { name: /Models/ }));
    expect(onChange).toHaveBeenCalledWith('library');
  });
});

describe('HelpMark', () => {
  it('has a name, is reachable by keyboard, and carries the tip', async () => {
    render(<HelpMark label="About objects" tip="Click an object to add notes." />);
    await userEvent.tab();
    const mark = screen.getByRole('button', { name: 'About objects' });
    expect(mark).toHaveFocus();
    expect(screen.getByRole('tooltip')).toHaveTextContent('Click an object to add notes.');
  });
});

describe('Tip', () => {
  it('runs its action and dismisses only when asked to', async () => {
    const onAction = vi.fn();
    const onDismiss = vi.fn();
    const { rerender } = render(
      <Tip action={{ label: 'Show me', onClick: onAction }} onDismiss={onDismiss}>
        Click an object to add notes.
      </Tip>
    );
    expect(screen.getByRole('note')).toHaveTextContent('Click an object to add notes.');
    await userEvent.click(screen.getByRole('button', { name: 'Show me' }));
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss tip' }));
    expect(onAction).toHaveBeenCalledOnce();
    expect(onDismiss).toHaveBeenCalledOnce();
    rerender(<Tip>Plain hint</Tip>);
    expect(screen.queryByRole('button')).toBeNull();
  });
});
