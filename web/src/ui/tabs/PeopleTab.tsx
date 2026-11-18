import { formatMoney, formatSimDate } from '../../logic/format';
import { useGameClient } from '../../state/useGameClient';
import { Chip } from '../Chips';

/** Productivity multiplier 1 − 0.75·e^(−h/80), mirroring fsim-people's ramp-up curve (display only). */
function rampPct(hours: number): number {
  return Math.round((1 - 0.75 * Math.exp(-hours / 80)) * 100);
}

export function PeopleTab() {
  const client = useGameClient();
  const world = client.world!;
  const plantId = client.selectedPlantId;
  if (!plantId) return <div className="card">Lease a plant first to hire people.</div>;
  const workers = Object.values(world.workers);

  const hire = () =>
    client.send({
      HireWorker: {
        plant_id: plantId,
        worker_id: `worker-${workers.length + 1}`,
        role: 'Multi-cell operator & logistics',
        wage_per_hour_tenth_cents: 32_000,
      },
    });

  return (
    <div className="stack">
      <section className="card row-between">
        <div>
          <h4>Hire an operator</h4>
          <p className="muted small">Operators carry an iPad, take one task at a time without interruption, and ramp up with experience.</p>
        </div>
        <button className="btn btn-primary" disabled={client.busy} onClick={() => void hire()}>
          + Hire ($32/h)
        </button>
      </section>

      <section className="card">
        <h4>Roster ({workers.length})</h4>
        {workers.length === 0 ? (
          <p className="muted">Nobody hired yet.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Role</th>
                <th>Wage</th>
                <th>Ramp-up</th>
                <th>iPad task</th>
              </tr>
            </thead>
            <tbody>
              {workers.map((w) => {
                const hours = w.worked_ms_in_domain / 3_600_000;
                const onboarded = world.now >= w.ready_at;
                return (
                  <tr key={w.worker_id}>
                    <td>
                      <strong>{w.name}</strong> <span className="muted small">{w.worker_id}</span>
                    </td>
                    <td>{w.role}</td>
                    <td>{formatMoney(w.wage_per_hour, client.currency)}/h</td>
                    <td>
                      {rampPct(hours)}% <span className="muted small">({hours.toFixed(1)} h)</span>
                    </td>
                    <td>
                      {!onboarded ? (
                        <Chip tone="info">Starts {formatSimDate(w.ready_at)}</Chip>
                      ) : w.active_task ? (
                        <Chip tone="ok">
                          {w.active_task.task.category} · {w.active_task.task.order_id}
                        </Chip>
                      ) : (
                        <Chip>Idle</Chip>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>

      <section className="card">
        <h4>Task priority order</h4>
        <div className="row wrap">
          {world.shift_priorities.map((p, i) => (
            <span key={p} className="pill">
              <span className="pill-label">#{i + 1}</span>
              {p}
            </span>
          ))}
        </div>
      </section>
    </div>
  );
}
