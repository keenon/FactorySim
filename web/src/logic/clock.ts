// Smooth, monotonic display clock for the 3D view.
//
// The server owns sim time; while playing, the client steps it every tick. Between server
// snapshots the view advances its own display time at `speed` so walking looks continuous.
// Worker trips are planned (timed segments known in advance), so rendering slightly ahead of the
// last snapshot is still exact. Display time never runs more than `leadCapMs` ahead of the server
// and snaps to the server after rewinds, skips, or pauses.

import type { SimTimeMs } from '../types';

export function nextDisplayTime(
  prevDisplayMs: SimTimeMs,
  serverNowMs: SimTimeMs,
  dtWallMs: number,
  speed: number,
  playing: boolean,
  leadCapMs: number,
): SimTimeMs {
  if (!playing) return serverNowMs;
  // Snap on rewind or when far behind (e.g. after a multi-day skip).
  if (prevDisplayMs > serverNowMs + leadCapMs || prevDisplayMs < serverNowMs - 2 * leadCapMs) return serverNowMs;
  const advanced = prevDisplayMs + Math.max(0, dtWallMs) * speed;
  return Math.min(advanced, serverNowMs + leadCapMs);
}
