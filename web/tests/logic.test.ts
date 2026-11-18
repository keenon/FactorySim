import test from 'node:test';
import assert from 'node:assert/strict';
import { formatDimension, formatMoney, formatSimDate } from '../src/logic/format';
import { poseAt } from '../src/logic/motion';
import { findFreeSpot, footprintRect, previewPlacementErrors, rotatedSize, type PlacementDraft } from '../src/logic/placement';
import { cellTransform, originFromCenter } from '../src/scene/coords';
import { LISTING } from './fixtures';



const draft = (over: Partial<PlacementDraft> = {}): PlacementDraft => ({
  draft_id: 'd1',
  plant_id: 'p',
  catalog_id: 'cnc',
  x_mm: 2000,
  y_mm: 2000,
  rotation_deg: 0,
  width_mm: 2400,
  depth_mm: 2000,
  height_mm: 2400,
  clearance_mm: 400,
  display_name: 'VMC',
  ...over,
});

test('formatting respects currency, units and sim clock', () => {
  assert.equal(formatMoney(500_000_000), '$500,000');
  assert.equal(formatMoney(-4_500, 'EUR'), '-€4.50');
  assert.equal(formatDimension(3048, 'imperial'), '10.0 ft');
  assert.equal(formatSimDate(86_400_000 + 3_600_000 * 5 + 60_000 * 7), 'Day 2, 05:07');
});

test('poseAt interpolates timed segments like fsim-motion', () => {
  const segs = [
    { start_ms: 1000, end_ms: 2000, start_pose: { x_mm: 0, y_mm: 0, heading_mdeg: 0 }, end_pose: { x_mm: 1200, y_mm: 0, heading_mdeg: 0 } },
  ];
  assert.deepEqual(poseAt(segs, 1500), { x_mm: 600, y_mm: 0, heading_mdeg: 0 });
  assert.deepEqual(poseAt(segs, 9000), { x_mm: 1200, y_mm: 0, heading_mdeg: 0 });
  assert.equal(poseAt([], 0), null);
});

test('rotated footprints swap width and depth at 90/270 like AabbMm::from_origin_size', () => {
  assert.deepEqual(rotatedSize(2400, 2000, 90), [2000, 2400]);
  assert.deepEqual(rotatedSize(2400, 2000, -180), [2400, 2000]);
  assert.deepEqual(footprintRect(100, 200, 2400, 2000, 270, 50), { minX: 50, minY: 150, maxX: 2150, maxY: 2650 });
});

test('placement preview flags walls, columns and overlaps, and findFreeSpot avoids them', () => {
  assert.deepEqual(previewPlacementErrors(draft(), LISTING, [], [], []), []);
  assert.match(previewPlacementErrors(draft({ x_mm: 19000 }), LISTING, [], [], [])[0], /outside building/);
  assert.match(previewPlacementErrors(draft({ x_mm: 8000, y_mm: 6500 }), LISTING, [], [], []).join(), /column/);
  const other = draft({ draft_id: 'd2', x_mm: 3000 });
  assert.match(previewPlacementErrors(draft(), LISTING, [], [], [other]).join(), /Overlaps planned/);

  const spot = findFreeSpot(draft(), LISTING, [], [], [draft({ draft_id: 'd0', x_mm: 400, y_mm: 400 })]);
  assert.deepEqual(previewPlacementErrors({ ...draft(), ...spot }, LISTING, [], [], [draft({ draft_id: 'd0', x_mm: 400, y_mm: 400 })]), []);
});

test('cellTransform centers on the rotated footprint and originFromCenter inverts it', () => {
  for (const rot of [0, 90, 180, 270]) {
    const tf = cellTransform(3000, 4000, 2400, 2000, rot);
    assert.deepEqual(originFromCenter(tf.cx, tf.cy, 2400, 2000, rot), { x_mm: 3000, y_mm: 4000 });
  }
  const tf90 = cellTransform(0, 0, 2400, 2000, 90);
  assert.ok(Math.abs(tf90.cx - 1.0) < 1e-9 && Math.abs(tf90.cy - 1.2) < 1e-9);
});
