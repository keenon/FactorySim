// Pure display formatting helpers.

import type { SimTimeMs, TenthCents } from '../types';

export type Currency = 'USD' | 'EUR';
export type UnitSystem = 'metric' | 'imperial';

export function formatMoney(tenthCents: TenthCents, currency: Currency = 'USD'): string {
  const dollars = (tenthCents || 0) / 1000;
  const symbol = currency === 'EUR' ? '€' : '$';
  const sign = dollars < 0 ? '-' : '';
  const abs = Math.abs(dollars);
  const digits = abs >= 1000 ? 0 : 2;
  return `${sign}${symbol}${abs.toLocaleString('en-US', {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })}`;
}

export function formatDimension(mm: number, units: UnitSystem = 'metric'): string {
  if (units === 'imperial') {
    return `${((mm || 0) / 304.8).toFixed(1)} ft`;
  }
  return `${((mm || 0) / 1000).toFixed(1)} m`;
}

export function formatSimDate(ms: SimTimeMs): string {
  const t = ms || 0;
  const day = Math.floor(t / 86_400_000) + 1;
  const hour = Math.floor((t % 86_400_000) / 3_600_000);
  const minute = Math.floor((t % 3_600_000) / 60_000);
  return `Day ${day}, ${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
}
