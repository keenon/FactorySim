import test from 'node:test';
import assert from 'node:assert/strict';
import type { Api } from '../src/api';
import { GameClient } from '../src/state/GameClient';
import type { CellCatalogEntry, ClientGameSnapshot, PlantCommand } from '../src/types';
import { LISTING } from './fixtures';

const VMC = {
  catalog_id: 'cnc-3axis-vmc',
  team: 'Machining',
  display_name: '3-Axis VMC',
  price: 65_000_000,
  width_mm: 2400,
  depth_mm: 2000,
  height_mm: 2400,
  clearance_mm: 400,
} as CellCatalogEntry;

function snapshot(): ClientGameSnapshot {
  return {
    world: {
      game_id: 'g1',
      now: 0,
      plants: { 'plant-1': { plant_id: 'plant-1', listing_id: 'b', name: 'P1' } },
      cells: {},
      workers: {},
    },
    cell_statuses: [],
    scene_nodes_by_plant: {},
  } as unknown as ClientGameSnapshot;
}

function fakeApi(sent: PlantCommand[], verdict = { valid: true, errors: [] as string[] }): Api {
  return {
    command: async (_id: string, cmd: PlantCommand) => {
      sent.push(cmd);
      return snapshot();
    },
    validatePlacement: async () => verdict,
  } as unknown as Api;
}

function client(api: Api): GameClient {
  const c = new GameClient(api);
  c.catalogs = { buildings: [LISTING], cells: [VMC], markets: [], certifications: [], preview_models: {} };
  c.applySnapshot(snapshot());
  return c;
}

test('addDraft places a valid ghost and selects it', () => {
  const c = client(fakeApi([]));
  const d = c.addDraft('cnc-3axis-vmc')!;
  assert.equal(c.drafts.length, 1);
  assert.equal(c.selectedEntityId, d.draft_id);
  assert.deepEqual(d.errors, []);
  assert.equal(c.plannedTotalTenthCents, 65_000_000);
});

test('live drag updates the scene without re-rendering React, snaps to 100 mm, and is undoable', () => {
  const c = client(fakeApi([]));
  const d = c.addDraft('cnc-3axis-vmc')!;
  const v = c.version;
  const sv = c.sceneVersion;
  c.beginDraftDrag();
  c.moveDraftLive(d.draft_id, 5_049, 6_151);
  assert.equal(c.version, v, 'no React re-render during drag');
  assert.ok(c.sceneVersion > sv, '3D scene notified');
  assert.deepEqual([d.x_mm, d.y_mm], [5_000, 6_200]);
  c.undo();
  assert.notEqual(c.drafts[0].x_mm, 5_000);
});

test('moving a ghost onto a wall marks it invalid and blocks commit', () => {
  const c = client(fakeApi([]));
  const d = c.addDraft('cnc-3axis-vmc')!;
  c.moveDraftLive(d.draft_id, 19_000, 2_000);
  assert.ok(c.anyInvalidDraft);
});

test('server verdict on drop overrides the local preview', async () => {
  const c = client(fakeApi([], { valid: false, errors: ['Needs 480V three-phase power'] }));
  const d = c.addDraft('cnc-3axis-vmc')!;
  await c.endDraftDrag(d.draft_id);
  assert.deepEqual(c.drafts[0].errors, ['Needs 480V three-phase power']);
});

test('commitDrafts sends one PlaceCell per ghost with its final pose', async () => {
  const sent: PlantCommand[] = [];
  const c = client(fakeApi(sent));
  const d = c.addDraft('cnc-3axis-vmc')!;
  c.moveDraftLive(d.draft_id, 6_000, 5_000);
  c.rotateDraft(d.draft_id);
  await c.commitDrafts();
  assert.equal(sent.length, 1);
  const place = (sent[0] as Extract<PlantCommand, { PlaceCell: unknown }>).PlaceCell;
  assert.deepEqual([place.catalog_id, place.x_mm, place.y_mm, place.rotation_deg], ['cnc-3axis-vmc', 6_000, 5_000, 90]);
  assert.equal(c.drafts.length, 0);
});
