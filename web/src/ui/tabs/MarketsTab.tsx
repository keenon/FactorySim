import { useState } from 'react';
import { formatMoney, formatSimDate } from '../../logic/format';
import type { MarketBiddingPolicy, MarketDefinition } from '../../types';
import { useGameClient } from '../../state/useGameClient';
import { Chip } from '../Chips';

export function MarketsTab() {
  const client = useGameClient();
  const world = client.world!;
  const plantId = client.selectedPlantId;
  if (!plantId) return <div className="card">Lease a plant first to configure its markets.</div>;

  const policyFor = (marketId: string): MarketBiddingPolicy | undefined =>
    world.market_policies.find(([[pid, mid]]) => pid === plantId && mid === marketId)?.[1];
  const orders = Object.values(world.customer_orders).slice(-15).reverse();

  return (
    <div className="stack">
      <h3>Markets</h3>
      <div className="grid-cards">
        {client.catalogs.markets.map((m) => (
          <MarketCard key={m.market_id} market={m} policy={policyFor(m.market_id)} />
        ))}
      </div>

      <h3>Recent customer orders ({Object.keys(world.customer_orders).length})</h3>
      <section className="card">
        {orders.length === 0 ? (
          <p className="muted">No orders yet. Turn on a market and skip ahead.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Order</th>
                <th>Part</th>
                <th>Stock</th>
                <th>Payout</th>
                <th>Due</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((o) => (
                <tr key={o.customer_order_id}>
                  <td>
                    <code>{o.customer_order_id}</code>
                  </td>
                  <td>{o.part_name}</td>
                  <td>
                    <code>{o.material_sku}</code>
                  </td>
                  <td>{formatMoney(o.payout, client.currency)}</td>
                  <td>{formatSimDate(o.due_at_ms)}</td>
                  <td>
                    <Chip tone={o.status === 'active' ? 'info' : o.status === 'shipped_on_time' ? 'ok' : 'warn'}>{o.status}</Chip>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <h3>Certifications</h3>
      <div className="grid-cards">
        {client.catalogs.certifications.map((c) => {
          const unlocked = world.unlocked_certs.includes(c.cert_id);
          return (
            <section key={c.cert_id} className="card">
              <div className="card-head">
                <strong>{c.name}</strong>
                <Chip tone={unlocked ? 'ok' : undefined}>{unlocked ? 'Registered' : c.company_wide ? 'Company-wide' : 'Per site'}</Chip>
              </div>
              <p className="muted small">{c.description}</p>
              <p className="small">
                {formatMoney(c.cost, client.currency)} · {c.lead_days} days
              </p>
              <button
                className="btn btn-sm"
                disabled={unlocked || client.busy}
                onClick={() => void client.send({ UnlockMarketCert: { plant_id: plantId, cert_id: c.cert_id } })}
              >
                {unlocked ? 'Unlocked' : 'Start audit'}
              </button>
            </section>
          );
        })}
      </div>
    </div>
  );
}

function MarketCard({ market: m, policy }: { market: MarketDefinition; policy?: MarketBiddingPolicy }) {
  const client = useGameClient();
  const world = client.world!;
  const [price, setPrice] = useState(policy?.price_multiplier_pct ?? 100);
  const [lead, setLead] = useState(policy?.promised_lead_days ?? 7);
  const enabled = policy?.enabled ?? false;
  const missing = m.required_certs.filter((c) => !world.unlocked_certs.includes(c));
  const rep = world.market_reputation_permille[m.market_id] ?? 650;

  const apply = (on: boolean) =>
    client.send({
      ConfigureMarketPolicy: {
        plant_id: client.selectedPlantId!,
        market_id: m.market_id,
        enabled: on,
        min_payout_per_hour_tenth_cents: 80_000,
        price_multiplier_pct: price,
        promised_lead_days: lead,
      },
    });

  return (
    <section className={`card ${enabled ? 'card-ok' : ''}`}>
      <div className="card-head">
        <strong>{m.name}</strong>
        <Chip tone={enabled ? 'ok' : undefined}>{enabled ? 'Active' : 'Off'}</Chip>
      </div>
      <p className="muted small">{m.description}</p>
      <p className="small">
        Reference payout <strong>{formatMoney(m.reference_payout_per_part, client.currency)}</strong>/part · reputation {(rep / 10).toFixed(1)}%
      </p>
      {missing.length > 0 && <p className="small warn-text">Needs: {missing.join(', ')}</p>}
      <div className="grid-2">
        <label className="small">
          Price (% of reference)
          <input className="input" type="number" value={price} onChange={(e) => setPrice(Number(e.target.value))} />
        </label>
        <label className="small">
          Promised lead (days)
          <input className="input" type="number" value={lead} onChange={(e) => setLead(Number(e.target.value))} />
        </label>
      </div>
      <button
        className={`btn btn-sm ${enabled ? '' : 'btn-primary'}`}
        disabled={missing.length > 0 || client.busy}
        onClick={() => void apply(!enabled)}
      >
        {enabled ? 'Pause orders' : 'Turn on & accept orders'}
      </button>
    </section>
  );
}
