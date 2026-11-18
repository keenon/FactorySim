import { useState } from 'react';
import { formatDimension, formatMoney, formatSimDate } from '../logic/format';
import { useGameClient } from '../state/useGameClient';
import { StatusChip } from './Chips';

export function Inspector() {
  const client = useGameClient();
  const snap = client.snapshot!;
  const sel = client.selectedEntityId;
  const draft = client.drafts.find((d) => d.draft_id === sel);
  const cell = sel ? snap.world.cells[sel] : undefined;
  const status = snap.cell_statuses.find((s) => s.cell_id === sel);
  const u = client.units;

  return (
    <div className="inspector">
      <h3>Inspector</h3>
      {draft ? (
        <section className={`card ${draft.errors.length ? 'card-bad' : 'card-accent'}`}>
          <div className="card-head">
            <strong>{draft.display_name}</strong>
            <span className={`chip ${draft.errors.length ? 'chip-bad' : 'chip-info'}`}>{draft.errors.length ? 'Blocked spot' : 'Planned'}</span>
          </div>
          <dl className="kv">
            <dt>Footprint</dt>
            <dd>
              {formatDimension(draft.width_mm, u)} × {formatDimension(draft.depth_mm, u)} × {formatDimension(draft.height_mm, u)}
            </dd>
            <dt>Position</dt>
            <dd>
              {formatDimension(draft.x_mm, u)}, {formatDimension(draft.y_mm, u)} · {draft.rotation_deg}°
            </dd>
            <dt>Price</dt>
            <dd>{formatMoney(draft.price_tenth_cents, client.currency)}</dd>
          </dl>
          {draft.errors.length > 0 ? (
            <ul className="errors">
              {draft.errors.map((e) => (
                <li key={e}>{e}</li>
              ))}
            </ul>
          ) : (
            <p className="muted small">Drag it on the floor to reposition. Nothing is ordered until you commit the plan.</p>
          )}
          <div className="row">
            <button className="btn btn-sm" onClick={() => client.rotateDraft(draft.draft_id)}>
              Rotate (R)
            </button>
            <button className="btn btn-sm btn-danger" onClick={() => client.removeDraft(draft.draft_id)}>
              Remove (Del)
            </button>
          </div>
        </section>
      ) : cell ? (
        <section className="card card-accent">
          <div className="card-head">
            <strong>{cell.display_name}</strong>
            {status && <StatusChip state={status.state} />}
          </div>
          {status && (
            <div className="why">
              <div className="why-title">Why isn't this running?</div>
              <div>{status.message}</div>
              <div className="muted small">
                {status.reason}
                {status.blocker_link ? ` · blocked by ${status.blocker_link}` : ''} · since {formatSimDate(status.since)}
              </div>
            </div>
          )}
          <dl className="kv">
            <dt>Cell</dt>
            <dd>
              <code>{cell.cell_id}</code> · {cell.team}
            </dd>
            <dt>Device</dt>
            <dd>
              <code>{cell.device_id}</code>
            </dd>
            <dt>Cycles</dt>
            <dd>{cell.cycles_completed}</dd>
            <dt>Tool wear</dt>
            <dd>
              <Meter pct={cell.tool_wear_pct} />
            </dd>
            <dt>Chip hopper</dt>
            <dd>
              <Meter pct={cell.chip_hopper_pct} />
            </dd>
          </dl>
          <p className="muted small">Drag the machine on the floor to relocate it ($1,500 rigging).</p>
        </section>
      ) : (
        <p className="muted small">Click a machine or planned ghost in the 3D view to inspect it.</p>
      )}

      <h4>All cells</h4>
      <div className="cell-list">
        {snap.cell_statuses.length === 0 && <p className="muted small">No cells yet.</p>}
        {snap.cell_statuses.map((s) => (
          <button key={s.cell_id} className={`cell-row ${sel === s.cell_id ? 'active' : ''}`} onClick={() => client.select(s.cell_id)}>
            <span>
              <code>{s.cell_id}</code>
              <span className="muted small"> {s.message}</span>
            </span>
            <StatusChip state={s.state} />
          </button>
        ))}
      </div>

      <Checkpoints />
    </div>
  );
}

function Meter({ pct }: { pct: number }) {
  return (
    <span className="meter">
      <span className={`meter-fill ${pct > 80 ? 'hot' : ''}`} style={{ width: `${Math.min(100, pct)}%` }} />
      <span className="meter-label">{pct}%</span>
    </span>
  );
}

function Checkpoints() {
  const client = useGameClient();
  const [name, setName] = useState('');
  const cks = client.snapshot!.checkpoints;
  return (
    <section className="card">
      <h4>Checkpoints</h4>
      <div className="row">
        <input className="input" placeholder="Name…" value={name} onChange={(e) => setName(e.target.value)} />
        <button
          className="btn btn-primary btn-sm"
          onClick={() => {
            void client.saveCheckpoint(name || 'Manual checkpoint');
            setName('');
          }}
        >
          Save
        </button>
      </div>
      <div className="ck-list">
        {[...cks].reverse().map((ck) => (
          <div key={ck.checkpoint_id} className="ck-row">
            <div>
              <div>{ck.name}</div>
              <div className="muted small">
                {formatSimDate(ck.sim_time_ms)} · {formatMoney(ck.cash_tenth_cents, client.currency)}
              </div>
            </div>
            <button className="btn btn-sm" onClick={() => void client.rewind(ck.checkpoint_id)}>
              Rewind
            </button>
          </div>
        ))}
      </div>
    </section>
  );
}
