import { Fragment } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface Crumb {
  label: string;
  onClick?: () => void;
}

export function Breadcrumb({ items, className }: { items: readonly Crumb[]; className?: string }) {
  return (
    <ol
      aria-label="Breadcrumb"
      className={cn(
        'm-0 flex list-none items-center gap-1.5 p-0 text-meta text-fg-mute',
        className
      )}
    >
      {items.map((it, i) => {
        const last = i === items.length - 1;
        return (
          <Fragment key={`${i}-${it.label}`}>
            {i > 0 ? (
              <li aria-hidden="true" className="flex">
                <Icon name="caret-right" size={11} />
              </li>
            ) : null}
            <li>
              {last ? (
                <span aria-current="page" className="text-fg">
                  {it.label}
                </span>
              ) : (
                <button
                  type="button"
                  onClick={it.onClick}
                  className="cursor-pointer text-fg-dim hover:text-fg"
                >
                  {it.label}
                </button>
              )}
            </li>
          </Fragment>
        );
      })}
    </ol>
  );
}
