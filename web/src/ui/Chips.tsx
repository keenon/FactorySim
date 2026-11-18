import type { CellRunState } from '../types';

const CLASS: Record<CellRunState, string> = {
  Running: 'chip-ok',
  Installing: 'chip-info',
  Planned: 'chip-info',
  Setup: 'chip-warn',
  Blocked: 'chip-warn',
  Down: 'chip-bad',
  Idle: '',
};

export function StatusChip({ state }: { state: CellRunState }) {
  return <span className={`chip ${CLASS[state]}`}>{state}</span>;
}

export function Chip({ tone, children }: { tone?: 'ok' | 'info' | 'warn' | 'bad' | 'accent'; children: React.ReactNode }) {
  return <span className={`chip ${tone ? `chip-${tone}` : ''}`}>{children}</span>;
}
