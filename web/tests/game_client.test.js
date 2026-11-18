import test from 'node:test';
import assert from 'node:assert/strict';
import {
  poseAt,
  formatMoney,
  formatDimension,
  project3DPoint,
  unprojectScreenToFloorMm,
  packScenePrimitives,
  hitTestPrimitiveAtScreen,
  GameClient,
} from '../dist/game_client.js';

test('poseAt interpolates timed motion segments identically to Rust fsim-motion', () => {
  const segments = [
    {
      start_ms: 1000,
      end_ms: 2000,
      start_pose: { x_mm: 0, y_mm: 0, heading_mdeg: 0 },
      end_pose: { x_mm: 1200, y_mm: 0, heading_mdeg: 0 },
    },
    {
      start_ms: 2000,
      end_ms: 3000,
      start_pose: { x_mm: 1200, y_mm: 0, heading_mdeg: 90000 },
      end_pose: { x_mm: 1200, y_mm: 1200, heading_mdeg: 90000 },
    },
  ];

  const mid = poseAt(segments, 1500);
  assert.equal(mid.x_mm, 600);
  assert.equal(mid.y_mm, 0);

  const after = poseAt(segments, 4000);
  assert.equal(after.x_mm, 1200);
  assert.equal(after.y_mm, 1200);
});

test('project3DPoint and unprojectScreenToFloorMm are exact inverses on the z=0 floor plane', () => {
  const camera = { panX: 340, panY: 110, zoom: 0.025, yawRad: 0.52, pitchRad: 0.85 };
  const orig = { x_mm: 6400, y_mm: 4800 };
  const proj = project3DPoint(orig.x_mm, orig.y_mm, 0, camera);
  const unproj = unprojectScreenToFloorMm(proj.sx, proj.sy, camera);
  assert.ok(Math.abs(unproj.x_mm - orig.x_mm) <= 2);
  assert.ok(Math.abs(unproj.y_mm - orig.y_mm) <= 2);
});

test('formatMoney and formatDimension respect currency and unit system settings', () => {
  assert.equal(formatMoney(500_000_000, 'USD'), '$500,000');
  assert.equal(formatMoney(4_500_000, 'EUR'), '€4,500');
  assert.equal(formatDimension(20000, 'metric'), '20.0 m');
  assert.equal(formatDimension(3048, 'imperial'), '10.0 ft');
});

test('GameClient OOP planning mode supports instant 3D ghost drafts, dragging, hit testing, and undo stack', () => {
  const client = new GameClient();
  client.applyServerSnapshot({
    world: {
      now: 0,
      plants: {
        'plant-1': {
          plant_id: 'plant-1',
          listing_id: 'bldg-1',
          name: 'Plant 1',
        },
      },
      cells: {},
      workers: {},
    },
    scene_nodes_by_plant: { 'plant-1': [] },
  });
  assert.equal(client.selectedPlantId, 'plant-1');

  const draft = client.addPlannedCellDraft(
    {
      catalog_id: 'cnc-3axis-vmc',
      team: 'Machining',
      display_name: '3-Axis VMC',
      price: 65_000_000,
      width_mm: 2400,
      depth_mm: 2000,
      height_mm: 2400,
      clearance_mm: 400,
    },
    5000,
    4000,
    0
  );

  assert.equal(client.plannedCells.length, 1);
  assert.equal(client.plannedTotalCostTenthCents(), 65_000_000);

  // Move draft and verify coordinates update synchronously
  client.movePlannedCellDraft(draft.draft_id, 7200, 5600);
  assert.equal(client.plannedCells[0].x_mm, 7200);
  assert.equal(client.plannedCells[0].y_mm, 5600);

  const primitives = packScenePrimitives(
    client.snapshot,
    'plant-1',
    client.plannedCells,
    0
  );
  assert.equal(primitives.length, 1);
  assert.equal(primitives[0].ghost, true);

  // Verify 3D hit-testing selects the box when clicking its roof center
  const roofCenter = project3DPoint(7200 + 1200, 5600 + 1000, 2400, client.camera);
  const hit = hitTestPrimitiveAtScreen(primitives, roofCenter.sx, roofCenter.sy, client.camera);
  assert.ok(hit);
  assert.equal(hit.id, draft.draft_id);

  assert.equal(client.undoPlannedChange(), true);
  assert.equal(client.plannedCells.length, 0);
});
