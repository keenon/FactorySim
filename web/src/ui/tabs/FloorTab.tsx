import { useEffect, useRef } from 'react';
import { formatDimension, formatMoney } from '../../logic/format';
import type { TeamTag } from '../../types';
import { useGameClient } from '../../state/useGameClient';
import { useFloorScene } from '../floorSceneContext';

const PALETTE_TEAMS: { team: TeamTag; label: string }[] = [
  { team: 'Machining', label: 'Machining' },
  { team: 'FinishingPacking', label: 'Packing' },
  { team: 'MaterialsLogistics', label: 'Materials' },
  { team: 'Quality', label: 'Quality' },
  { team: 'Molding', label: 'Molding' },
  { team: 'Toolroom', label: 'Toolroom' },
  { team: 'HeatTreat', label: 'Heat treat' },
  { team: 'FacilitiesEhs', label: 'Facilities' },
];

export function FloorTab() {
  const client = useGameClient();
  const world = client.world!;
  const plant = client.selectedPlantId ? world.plants[client.selectedPlantId] : undefined;
  if (!plant) {
    return <div className="card">Lease a plant on the Plants tab first.</div>;
  }
  return (
    <div className="floor-layout">
      <Palette />
      <div className="viewport-card">
        <FloorToolbar plantName={plant.name} />
        <Viewport />
        <div className="viewport-hint">
          Space play/pause · N next event · Left-drag orbit · Right-drag pan · Wheel zoom · WASD move · Drag a machine or ghost to move it · R rotate · Del remove · Hover for names
        </div>
      </div>
    </div>
  );
}

function Viewport() {
  const scene = useFloorScene();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    scene.mount(el);
    return () => scene.unmount();
  }, [scene]);
  return <div className="viewport" ref={ref} />;
}

function FloorToolbar({ plantName }: { plantName: string }) {
  const client = useGameClient();
  const scene = useFloorScene();
  const n = client.drafts.length;
  return (
    <div className="viewport-toolbar">
      <strong>{plantName}</strong>
      <div className="row">
        <button className="btn btn-sm" onClick={() => scene.resetView('game')}>
          Reset camera
        </button>
        <button className="btn btn-sm" onClick={() => scene.resetView('top')}>
          Top view
        </button>
        {n > 0 && (
          <>
            <span className="small">
              Plan: <strong>{formatMoney(client.plannedTotalTenthCents, client.currency)}</strong> · {n} item{n > 1 ? 's' : ''}
            </span>
            <button className="btn btn-sm" disabled={!client.canUndo} onClick={() => client.undo()}>
              Undo
            </button>
            <button className="btn btn-sm" onClick={() => client.clearDrafts()}>
              Clear
            </button>
            <button
              className="btn btn-sm btn-primary"
              disabled={client.anyInvalidDraft || client.busy}
              title={client.anyInvalidDraft ? 'Move red ghosts to a valid spot first' : 'Order everything in the plan'}
              onClick={() => void client.commitDrafts()}
            >
              Commit plan
            </button>
          </>
        )}
        <button
          className="btn btn-sm"
          disabled={client.busy}
          onClick={async () => {
            const plant_id = client.selectedPlantId!;
            await client.send({ OrderRawStock: { plant_id, sku: 'blank-alu', quantity: 6 } });
            await client.send({ OrderRawStock: { plant_id, sku: 'carton-box', quantity: 6 } });
          }}
        >
          + Stock
        </button>
      </div>
    </div>
  );
}

function Palette() {
  const client = useGameClient();
  const listing = client.currentListing();
  const plant = client.world!.plants[client.selectedPlantId!];
  const items = client.catalogs.cells.filter((c) => c.team === client.paletteTeam);
  const u = client.units;
  return (
    <div className="palette">
      <div className="palette-teams">
        {PALETTE_TEAMS.map(({ team, label }) => (
          <button key={team} className={`chip-btn ${client.paletteTeam === team ? 'active' : ''}`} onClick={() => client.setPaletteTeam(team)}>
            {label}
          </button>
        ))}
      </div>
      <div className="palette-items">
        {items.map((c) => {
          let blocked: string | null = null;
          if (listing && c.height_mm > listing.clear_height_mm) blocked = `Needs ${formatDimension(c.height_mm, u)} clear height`;
          else if (c.needs_climate_room && !plant.climate_room_built) blocked = 'Needs a climate-controlled room';
          else if (c.needs_hazmat_permit && !plant.hazmat_permit_approved) blocked = 'Needs a hazmat permit';
          const has3d = Boolean(client.catalogs.preview_models[c.catalog_id]);
          return (
            <div key={c.catalog_id} className={`palette-item ${blocked ? 'disabled' : ''}`}>
              <div className="palette-title">
                {c.display_name}
                {has3d && <span className="chip chip-accent">3D</span>}
              </div>
              <div className="muted small">{c.description}</div>
              <div className="small">
                <strong>{formatMoney(c.price, client.currency)}</strong> · {formatDimension(c.width_mm, u)} × {formatDimension(c.depth_mm, u)} · {c.lead_time_days_min}–
                {c.lead_time_days_max} d {c.financeable ? '· financeable' : ''}
              </div>
              <button className="btn btn-sm wide" disabled={Boolean(blocked)} onClick={() => client.addDraft(c.catalog_id)}>
                {blocked ?? '+ Add to plan'}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
