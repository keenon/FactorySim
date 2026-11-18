// Pure helpers over the server's precomputed transport graph (fsim-layout-graph routes exist
// only for material flows: output square -> compatible input square of another cell).

import type { LayoutRoute, PointMm, SimWorld } from '../types';

export type RouteRole = 'idle' | 'in' | 'out';

export function plantRoutes(world: SimWorld | null, plantId: string | null): LayoutRoute[] {
  if (!world || !plantId) return [];
  const graph = world.transport_graphs[plantId];
  return graph ? graph.routes.map(([, r]) => r) : [];
}

/** `in` = feeds the selected cell (upstream), `out` = leaves it (downstream). */
export function routeRole(route: LayoutRoute, selectedCellId: string | null): RouteRole {
  if (!selectedCellId) return 'idle';
  if (route.to_cell === selectedCellId) return 'in';
  if (route.from_cell === selectedCellId) return 'out';
  return 'idle';
}

export function cellFlows(routes: readonly LayoutRoute[], cellId: string): { upstream: LayoutRoute[]; downstream: LayoutRoute[] } {
  return {
    upstream: routes.filter((r) => r.to_cell === cellId),
    downstream: routes.filter((r) => r.from_cell === cellId),
  };
}

/**
 * Flat triangle strip (as a triangle list, xyz in meters at height `z`) for a polyline of mm points,
 * with mitered joins capped at 3x half-width so sharp turns stay bounded.
 */
export function ribbonTriangles(points: readonly PointMm[], widthMm: number, z: number): number[] {
  const pts = dedupe(points);
  if (pts.length < 2) return [];
  const half = widthMm / 2;
  const left: [number, number][] = [];
  const right: [number, number][] = [];
  for (let i = 0; i < pts.length; i++) {
    const prev = pts[Math.max(0, i - 1)];
    const next = pts[Math.min(pts.length - 1, i + 1)];
    const d0 = unit(i === 0 ? sub(next, pts[i]) : sub(pts[i], prev));
    const d1 = unit(i === pts.length - 1 ? sub(pts[i], prev) : sub(next, pts[i]));
    let nx = -(d0[1] + d1[1]);
    let ny = d0[0] + d1[0];
    const nl = Math.hypot(nx, ny) || 1;
    nx /= nl;
    ny /= nl;
    // Miter length = half / cos(theta/2), capped.
    const cos = Math.max(1 / 3, nx * -d1[1] + ny * d1[0]);
    const m = half / cos;
    left.push([pts[i].x + nx * m, pts[i].y + ny * m]);
    right.push([pts[i].x - nx * m, pts[i].y - ny * m]);
  }
  const out: number[] = [];
  const v = (p: [number, number]) => out.push(p[0] / 1000, p[1] / 1000, z);
  for (let i = 0; i < pts.length - 1; i++) {
    v(left[i]);
    v(right[i]);
    v(left[i + 1]);
    v(left[i + 1]);
    v(right[i]);
    v(right[i + 1]);
  }
  return out;
}

/** Flat arrow chevrons every `spacingMm` along the polyline, pointing in travel direction. */
export function chevronTriangles(points: readonly PointMm[], spacingMm: number, sizeMm: number, z: number): number[] {
  const pts = dedupe(points);
  const out: number[] = [];
  let carry = spacingMm / 2;
  for (let i = 0; i < pts.length - 1; i++) {
    const a = pts[i];
    const seg = sub(pts[i + 1], a);
    const len = Math.hypot(seg[0], seg[1]);
    const [dx, dy] = unit(seg);
    for (let s = carry; s < len; s += spacingMm) {
      const cx = a.x + dx * s;
      const cy = a.y + dy * s;
      const tip = [cx + dx * sizeMm, cy + dy * sizeMm];
      const l = [cx - dy * sizeMm * 0.8, cy + dx * sizeMm * 0.8];
      const r = [cx + dy * sizeMm * 0.8, cy - dx * sizeMm * 0.8];
      out.push(tip[0] / 1000, tip[1] / 1000, z, l[0] / 1000, l[1] / 1000, z, r[0] / 1000, r[1] / 1000, z);
    }
    carry = (carry - len) % spacingMm;
    if (carry < 0) carry += spacingMm;
  }
  return out;
}

function dedupe(points: readonly PointMm[]): PointMm[] {
  const out: PointMm[] = [];
  for (const p of points) {
    const last = out[out.length - 1];
    if (!last || last.x !== p.x || last.y !== p.y) out.push(p);
  }
  return out;
}

function sub(a: PointMm, b: PointMm): [number, number] {
  return [a.x - b.x, a.y - b.y];
}

function unit(v: [number, number]): [number, number] {
  const l = Math.hypot(v[0], v[1]) || 1;
  return [v[0] / l, v[1] / l];
}
