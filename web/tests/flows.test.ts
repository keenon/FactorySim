import test from 'node:test';
import assert from 'node:assert/strict';
import type { Api } from '../src/api';
import { nextDisplayTime } from '../src/logic/clock';
import { cellFlows, chevronTriangles, ribbonTriangles, routeRole } from '../src/logic/flows';
import { GameClient } from '../src/state/GameClient';
import type { ClientGameSnapshot, LayoutRoute } from '../src/types';

const route = (from: string, to: string, waypoints = [{ x: 0, y: 0 }, { x: 4000, y: 0 }, { x: 4000, y: 3000 }]): LayoutRoute => ({
  from_cell: from,
  from_square: 'out',
  to_cell: to,
  to_square: 'part_in',
  waypoints,
  distance_mm: 7000,
  forklift_feasible: true,
});

test('display clock is smooth, capped ahead of the server, and snaps on rewind/pause', () => {
  // Paused: always the server time.
  assert.equal(nextDisplayTime(500, 1000, 16, 5, false, 2500), 1000);
  // Playing: advances by wall dt * speed.
  assert.equal(nextDisplayTime(1000, 1000, 100, 5, true, 2500), 1500);
  // Never more than the lead cap ahead of the server.
  assert.equal(nextDisplayTime(3400, 1000, 100, 5, true, 2500), 3500);
  // Snaps to the server after a rewind (display far ahead) or a big skip (far behind).
  assert.equal(nextDisplayTime(90_000, 1000, 16, 5, true, 2500), 1000);
  assert.equal(nextDisplayTime(1000, 900_000, 16, 5, true, 2500), 900_000);
});

test('routes are classified as upstream/downstream relative to the selected cell', () => {
  const rs = [route('recv', 'cnc'), route('cnc', 'pack'), route('pack', 'ship')];
  assert.deepEqual(rs.map((r) => routeRole(r, 'cnc')), ['in', 'out', 'idle']);
  assert.deepEqual(rs.map((r) => routeRole(r, null)), ['idle', 'idle', 'idle']);
  const f = cellFlows(rs, 'pack');
  assert.deepEqual([f.upstream.map((r) => r.from_cell), f.downstream.map((r) => r.to_cell)], [['cnc'], ['ship']]);
});

test('ribbon geometry is a flat strip of the requested width following the path', () => {
  const tri = ribbonTriangles([{ x: 0, y: 0 }, { x: 4000, y: 0 }], 200, 0.01);
  assert.equal(tri.length, 6 * 3); // one quad = two triangles
  const ys = tri.filter((_, i) => i % 3 === 1);
  assert.deepEqual([Math.min(...ys), Math.max(...ys)], [-0.1, 0.1]);
  assert.ok(tri.filter((_, i) => i % 3 === 2).every((z) => z === 0.01));
  // Duplicate points are ignored; a corner yields two quads.
  assert.equal(ribbonTriangles([{ x: 0, y: 0 }, { x: 0, y: 0 }, { x: 4000, y: 0 }, { x: 4000, y: 3000 }], 200, 0).length, 2 * 18);
  assert.deepEqual(ribbonTriangles([{ x: 1, y: 1 }], 200, 0), []);
});

test('chevrons are spaced along the whole polyline', () => {
  // 7 m path, 1 m spacing starting at 0.5 m -> 7 chevrons, 3 vertices each.
  const tri = chevronTriangles(route('a', 'b').waypoints, 1000, 100, 0);
  assert.equal(tri.length / 9, 7);
});

test('play loop steps the server by speed x tick and stops on pause', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval'] });
  const steps: number[] = [];
  const snap = { world: { game_id: 'g', now: 0, plants: {}, cells: {}, workers: {} }, runway: { bankrupt: false } } as unknown as ClientGameSnapshot;
  const api = {
    step: async (_id: string, ms: number) => {
      steps.push(ms);
      return snap;
    },
  } as unknown as Api;
  const c = new GameClient(api);
  c.applySnapshot(snap);
  c.setSpeed(30);
  assert.equal(c.playing, true);
  t.mock.timers.tick(GameClient.TICK_MS);
  await new Promise((r) => setImmediate(r));
  t.mock.timers.tick(GameClient.TICK_MS);
  await new Promise((r) => setImmediate(r));
  assert.deepEqual(steps, [30 * GameClient.TICK_MS, 30 * GameClient.TICK_MS]);
  c.setPlaying(false);
  t.mock.timers.tick(GameClient.TICK_MS * 4);
  await new Promise((r) => setImmediate(r));
  assert.equal(steps.length, 2);
});
