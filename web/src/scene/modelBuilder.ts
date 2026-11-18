// Builds Three.js objects from team-generated `CellModel`s (fsim-core). The Rust visualize
// functions own *what* a cell looks like; this module only maps primitives/materials to meshes.
// Groups are centered on the footprint with z = 0 at the floor.

import * as THREE from 'three';
import type { CellModel, ModelMaterial, ModelPart } from '../types';
import { MM } from './coords';

export type GhostStyle = 'none' | 'valid' | 'invalid' | 'installing';

const UNIT_BOX = new THREE.BoxGeometry(1, 1, 1);
const UNIT_CYL = new THREE.CylinderGeometry(1, 1, 1, 28);

const GHOST_TINT: Record<Exclude<GhostStyle, 'none'>, string> = {
  valid: '#6aa9d8',
  invalid: '#e07a6c',
  installing: '#a9b8c4',
};

const materialCache = new Map<string, THREE.Material>();

function material(kind: ModelMaterial, color: string, ghost: GhostStyle): THREE.Material {
  const key = `${kind}|${color}|${ghost}`;
  const hit = materialCache.get(key);
  if (hit) return hit;
  let m: THREE.MeshStandardMaterial;
  if (ghost !== 'none') {
    const tint = new THREE.Color(GHOST_TINT[ghost]).lerp(new THREE.Color(color), ghost === 'installing' ? 0.45 : 0.25);
    m = new THREE.MeshStandardMaterial({
      color: tint,
      transparent: true,
      opacity: ghost === 'installing' ? 0.55 : 0.42,
      roughness: 0.6,
      depthWrite: false,
      emissive: new THREE.Color(GHOST_TINT[ghost]),
      emissiveIntensity: 0.12,
    });
  } else {
    switch (kind) {
      case 'Metal':
        m = new THREE.MeshStandardMaterial({ color, metalness: 0.75, roughness: 0.32 });
        break;
      case 'Glass':
        m = new THREE.MeshStandardMaterial({ color, transparent: true, opacity: 0.35, roughness: 0.08, metalness: 0.1, depthWrite: false });
        break;
      case 'Rubber':
        m = new THREE.MeshStandardMaterial({ color, roughness: 0.95 });
        break;
      case 'Emissive':
        m = new THREE.MeshStandardMaterial({ color, emissive: new THREE.Color(color), emissiveIntensity: 0.9, roughness: 0.4 });
        break;
      case 'Screen':
        m = new THREE.MeshStandardMaterial({ color, emissive: new THREE.Color(color), emissiveIntensity: 0.45, roughness: 0.2 });
        break;
      default:
        m = new THREE.MeshStandardMaterial({ color, roughness: 0.55, metalness: 0.05 });
    }
  }
  materialCache.set(key, m);
  return m;
}

function partMesh(part: ModelPart, ghost: GhostStyle): THREE.Mesh {
  const mat = material(part.material, part.color_hex, ghost);
  let mesh: THREE.Mesh;
  if (part.shape.kind === 'Box') {
    mesh = new THREE.Mesh(UNIT_BOX, mat);
    const [sx, sy, sz] = part.shape.size_mm;
    mesh.scale.set(sx * MM, sy * MM, sz * MM);
  } else {
    // CylinderGeometry's axis is +Y; scale in local frame, then orient to the requested axis.
    mesh = new THREE.Mesh(UNIT_CYL, mat);
    const r = part.shape.radius_mm * MM;
    mesh.scale.set(r, part.shape.length_mm * MM, r);
    if (part.shape.axis === 'Z') mesh.rotation.x = Math.PI / 2;
    else if (part.shape.axis === 'X') mesh.rotation.z = -Math.PI / 2;
  }
  mesh.name = part.node;
  mesh.castShadow = ghost === 'none' && part.material !== 'Glass';
  mesh.receiveShadow = ghost === 'none';
  return mesh;
}

/** Builds a footprint-centered group from a team `CellModel`. */
export function buildCellModel(model: CellModel, ghost: GhostStyle = 'none'): THREE.Group {
  const group = new THREE.Group();
  const ox = -model.width_mm / 2;
  const oy = -model.depth_mm / 2;
  for (const part of model.parts) {
    const mesh = partMesh(part, ghost);
    mesh.position.set((part.center_mm[0] + ox) * MM, (part.center_mm[1] + oy) * MM, part.center_mm[2] * MM);
    group.add(mesh);
  }
  return group;
}

/** Core-drawn fallback for cells whose team has not shipped a model: a rounded-looking box with a base. */
export function buildFallbackBox(widthMm: number, depthMm: number, heightMm: number, color: string, ghost: GhostStyle = 'none'): THREE.Group {
  return buildCellModel(
    {
      width_mm: widthMm,
      depth_mm: depthMm,
      height_mm: heightMm,
      parts: [
        { node: 'base', shape: { kind: 'Box', size_mm: [widthMm, depthMm, 120] }, center_mm: [widthMm / 2, depthMm / 2, 60], material: 'Paint', color_hex: '#9aa3aa' },
        {
          node: 'body',
          shape: { kind: 'Box', size_mm: [widthMm - 80, depthMm - 80, heightMm - 120] },
          center_mm: [widthMm / 2, depthMm / 2, 120 + (heightMm - 120) / 2],
          material: 'Paint',
          color_hex: color,
        },
      ],
    },
    ghost,
  );
}

/** Rectangular floor outline (footprint and clearance halo) in the group's local frame. */
export function buildFootprintOutline(widthMm: number, depthMm: number, clearanceMm: number, color: string): THREE.Group {
  const g = new THREE.Group();
  const rect = (w: number, d: number, dashed: boolean, opacity: number) => {
    const hw = (w / 2) * MM;
    const hd = (d / 2) * MM;
    const geo = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(-hw, -hd, 0.006),
      new THREE.Vector3(hw, -hd, 0.006),
      new THREE.Vector3(hw, hd, 0.006),
      new THREE.Vector3(-hw, hd, 0.006),
      new THREE.Vector3(-hw, -hd, 0.006),
    ]);
    const mat = dashed
      ? new THREE.LineDashedMaterial({ color, dashSize: 0.15, gapSize: 0.1, transparent: true, opacity })
      : new THREE.LineBasicMaterial({ color, transparent: true, opacity });
    const line = new THREE.Line(geo, mat);
    if (dashed) line.computeLineDistances();
    return line;
  };
  g.add(rect(widthMm, depthMm, false, 0.95));
  g.add(rect(widthMm + 2 * clearanceMm, depthMm + 2 * clearanceMm, true, 0.8));
  const fill = new THREE.Mesh(
    new THREE.PlaneGeometry((widthMm + 2 * clearanceMm) * MM, (depthMm + 2 * clearanceMm) * MM),
    new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.12, depthWrite: false }),
  );
  fill.position.z = 0.004;
  g.add(fill);
  return g;
}
