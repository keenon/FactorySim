import test from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { buildCellModel } from '../src/scene/modelBuilder';
import type { CellModel } from '../src/types';

const MODEL: CellModel = {
  width_mm: 2400,
  depth_mm: 2000,
  height_mm: 2400,
  parts: [
    { node: 'base', shape: { kind: 'Box', size_mm: [2400, 2000, 260] }, center_mm: [1200, 1000, 130], material: 'Paint', color_hex: '#8a949c' },
    { node: 'spindle', shape: { kind: 'Cylinder', radius_mm: 80, length_mm: 400, axis: 'Z' }, center_mm: [1200, 900, 1500], material: 'Metal', color_hex: '#b8c0c6' },
    { node: 'chuck', shape: { kind: 'Cylinder', radius_mm: 100, length_mm: 600, axis: 'X' }, center_mm: [1200, 1000, 2000], material: 'Metal', color_hex: '#b8c0c6' },
  ],
};

function boxOf(o: THREE.Object3D): THREE.Box3 {
  o.updateMatrixWorld(true);
  return new THREE.Box3().setFromObject(o);
}

test('built model is centered on its footprint and stays inside the declared envelope', () => {
  const g = buildCellModel(MODEL);
  const b = boxOf(g);
  const eps = 1e-6;
  assert.ok(b.min.x >= -1.2 - eps && b.max.x <= 1.2 + eps);
  assert.ok(b.min.y >= -1.0 - eps && b.max.y <= 1.0 + eps);
  assert.ok(b.min.z >= -eps && b.max.z <= 2.4 + eps);
  assert.deepEqual(
    g.children.map((c) => c.name),
    ['base', 'spindle', 'chuck'],
  );
});

test('cylinder parts are oriented along their declared axis', () => {
  const g = buildCellModel(MODEL);
  const spindle = boxOf(g.getObjectByName('spindle')!);
  assert.ok(Math.abs(spindle.max.z - spindle.min.z - 0.4) < 1e-3, 'Z cylinder spans its length in z');
  const chuck = boxOf(g.getObjectByName('chuck')!);
  assert.ok(Math.abs(chuck.max.x - chuck.min.x - 0.6) < 1e-3, 'X cylinder spans its length in x');
  assert.ok(Math.abs(chuck.max.z - chuck.min.z - 0.2) < 2e-3);
});

test('ghost styles produce translucent materials', () => {
  const g = buildCellModel(MODEL, 'invalid');
  const mat = (g.children[0] as THREE.Mesh).material as THREE.MeshStandardMaterial;
  assert.equal(mat.transparent, true);
  assert.ok(mat.opacity < 1);
});
