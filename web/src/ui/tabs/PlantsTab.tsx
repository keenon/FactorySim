import { formatDimension, formatMoney, formatSimDate } from '../../logic/format';
import { polygonBounds } from '../../logic/placement';
import { useGameClient } from '../../state/useGameClient';
import { Chip } from '../Chips';

export function PlantsTab() {
  const client = useGameClient();
  const world = client.world!;
  const plants = Object.values(world.plants);
  const u = client.units;

  const lease = async (listingId: string, name: string) => {
    const n = plants.length + 1;
    const pid = `plant-${n}`;
    await client.send({ LeasePlant: { plant_id: pid, listing_id: listingId, name: `${name} (#${n})` } });
    client.selectPlant(pid);
  };

  return (
    <div className="stack">
      <h3>Your plants ({plants.length})</h3>
      {plants.length === 0 ? (
        <div className="notice">You have no plants yet. Pick a site below to sign a lease.</div>
      ) : (
        <div className="grid-cards">
          {plants.map((p) => {
            const ready = p.keys_received && p.utilities_ready;
            const selected = client.selectedPlantId === p.plant_id;
            return (
              <section key={p.plant_id} className={`card ${selected ? 'card-accent' : ''}`}>
                <div className="card-head">
                  <strong>{p.name}</strong>
                  <Chip tone={ready ? 'ok' : 'info'}>{ready ? 'Operational' : 'Build-out'}</Chip>
                </div>
                <p className="small">
                  Keys: <strong>{p.keys_received ? 'received' : `ETA ${formatSimDate(p.keys_ready_at)}`}</strong> · Utilities:{' '}
                  <strong>{p.utilities_ready ? 'ready' : `ETA ${formatSimDate(p.utilities_ready_at)}`}</strong>
                </p>
                <button className="btn btn-sm" disabled={selected} onClick={() => client.selectPlant(p.plant_id)}>
                  {selected ? 'Viewing' : 'View this plant'}
                </button>
              </section>
            );
          })}
        </div>
      )}

      <h3>Available sites</h3>
      <div className="grid-cards">
        {client.catalogs.buildings.map((b) => {
          const bb = polygonBounds(b.floor_polygon.outer);
          return (
            <section key={b.listing_id} className="card">
              <div className="card-head">
                <strong>{b.name}</strong>
                <Chip>{b.building_kind}</Chip>
              </div>
              <p className="muted small">{b.description}</p>
              <dl className="kv small">
                <dt>Floor</dt>
                <dd>
                  {formatDimension(bb.maxX - bb.minX, u)} × {formatDimension(bb.maxY - bb.minY, u)} · clear {formatDimension(b.clear_height_mm, u)}
                </dd>
                <dt>Door</dt>
                <dd>
                  {formatDimension(b.max_door_width_mm, u)} × {formatDimension(b.max_door_height_mm, u)} · {b.power_capacity_kw} kW
                </dd>
                <dt>Rent</dt>
                <dd>
                  {formatMoney(b.monthly_rent, client.currency)}/mo · deposit {formatMoney(b.lease_deposit, client.currency)}
                </dd>
                <dt>Lead time</dt>
                <dd>
                  keys {b.keys_lead_days_min}–{b.keys_lead_days_max} d, utilities {b.utilities_lead_days_min}–{b.utilities_lead_days_max} d
                </dd>
              </dl>
              <button className="btn btn-primary btn-sm" disabled={client.busy} onClick={() => void lease(b.listing_id, b.name)}>
                Sign lease
              </button>
            </section>
          );
        })}
      </div>
    </div>
  );
}
