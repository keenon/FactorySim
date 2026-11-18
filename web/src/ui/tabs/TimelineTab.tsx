import { formatMoney, formatSimDate } from '../../logic/format';
import { useGameClient } from '../../state/useGameClient';
import { Chip } from '../Chips';

export function TimelineTab() {
  const client = useGameClient();
  const world = client.world!;
  const pipes = Object.values(world.pipeline);
  const orders = Object.values(world.internal_orders).slice(-24).reverse();

  // Shared time axis across all pipeline bars.
  const t0 = Math.min(world.now, ...pipes.map((p) => p.started_at));
  const t1 = Math.max(world.now + 86_400_000, ...pipes.map((p) => p.eta_p90));
  const pct = (t: number) => ((t - t0) / Math.max(1, t1 - t0)) * 100;

  return (
    <div className="stack">
      <section className="card">
        <h3>Everything in flight</h3>
        {pipes.length === 0 && <p className="muted">Nothing in flight yet.</p>}
        <div className="waterfall">
          {pipes.map((p) => (
            <div key={p.item_id} className="wf-row">
              <div>
                <div>{p.title}</div>
                <div className="muted small">
                  {p.category} · {formatMoney(p.cost, client.currency)}
                </div>
              </div>
              <div className="wf-track">
                <div className="wf-now" style={{ left: `${pct(world.now)}%` }} />
                <div
                  className={`wf-band ${p.completed ? 'done' : ''}`}
                  style={{ left: `${pct(p.eta_p10)}%`, width: `${Math.max(1, pct(p.eta_p90) - pct(p.eta_p10))}%` }}
                />
                <div className="wf-p50" style={{ left: `${pct(p.eta_p50)}%` }} />
              </div>
              <div className="right">
                <Chip tone={p.completed ? 'ok' : 'info'}>{p.completed ? 'Done' : formatSimDate(p.eta_p50)}</Chip>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="card">
        <h3>Pull orders (shipping → packing → machining → stock)</h3>
        {orders.length === 0 ? (
          <p className="muted">No pull orders yet.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Order</th>
                <th>Feeds</th>
                <th>Team</th>
                <th>Cell</th>
                <th>Output</th>
                <th>Status</th>
                <th>Waiting on</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((o) => (
                <tr key={o.order_id}>
                  <td>
                    <code>{o.order_id}</code>
                  </td>
                  <td>
                    <code>{o.parent_order_id || 'customer'}</code>
                  </td>
                  <td>{o.target_team}</td>
                  <td>
                    <code>{o.target_cell_id || '—'}</code>
                  </td>
                  <td>
                    <code>{o.output_spec.item_sku}</code>
                  </td>
                  <td>
                    <Chip tone={o.status === 'Completed' ? 'ok' : o.status === 'BlockedUpstream' ? 'warn' : 'info'}>{o.status}</Chip>
                  </td>
                  <td className="small">{o.upstream_order_ids.join(', ') || '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
