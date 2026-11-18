import { formatMoney, formatSimDate } from '../../logic/format';
import { useGameClient } from '../../state/useGameClient';

export function LedgerTab() {
  const client = useGameClient();
  const snap = client.snapshot!;
  const { runway: rw, bottleneck: bn, world } = snap;
  const cur = client.currency;

  const borrow = () => {
    const collateral = Object.values(world.cells).find((c) => c.team === 'Machining' || c.team === 'Molding');
    if (!collateral) {
      client.showBanner('Install a financeable CNC or molding machine first.');
      return;
    }
    void client.send({
      TakeEquipmentLoan: {
        loan_id: `loan-${Object.keys(world.loans).length + 1}`,
        principal_tenth_cents: 45_000_000,
        collateral_cell_id: collateral.cell_id,
      },
    });
  };

  return (
    <div className="stack">
      <div className="grid-3">
        <section className="card">
          <h4>Cash & runway</h4>
          <div className="big pos">{formatMoney(rw.cash_now, cur)}</div>
          <dl className="kv small">
            <dt>Credit headroom</dt>
            <dd>{formatMoney(rw.credit_available, cur)}</dd>
            <dt>Gross burn</dt>
            <dd>{formatMoney(rw.gross_monthly_burn, cur)}/mo</dd>
            <dt>Runway P10/50/90</dt>
            <dd>
              {rw.runway_days_p10} / {rw.runway_days_p50} / {rw.runway_days_p90} d
            </dd>
          </dl>
        </section>
        <section className="card">
          <h4>Bottleneck</h4>
          <p className="accent-text">{bn.recommendation}</p>
          <dl className="kv small">
            <dt>Utilization</dt>
            <dd>{(bn.utilization_permille / 10).toFixed(1)}%</dd>
            <dt>Shadow value</dt>
            <dd>${bn.shadow_price_dollars_per_hr}/hr</dd>
            <dt>Gateway log</dt>
            <dd>{world.gateway.log_entries.length} entries</dd>
          </dl>
        </section>
        <section className="card">
          <h4>Equipment financing</h4>
          <p className="muted small">Borrow up to 80% against a resellable machine (8.5% APR, 36 months).</p>
          <button className="btn btn-primary btn-sm" disabled={client.busy} onClick={borrow}>
            Borrow $45,000
          </button>
        </section>
      </div>

      <div className="grid-ledger">
        <section className="card">
          <h4>Balances (sum = 0)</h4>
          <table className="data-table">
            <tbody>
              {Object.entries(world.ledger.account_balances).map(([acct, val]) => (
                <tr key={acct}>
                  <td>
                    <code>{acct}</code>
                  </td>
                  <td className="right">{formatMoney(val, cur)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
        <section className="card">
          <h4>Journal</h4>
          <table className="data-table">
            <thead>
              <tr>
                <th>Date</th>
                <th>Debit</th>
                <th>Credit</th>
                <th className="right">Amount</th>
                <th>Memo</th>
              </tr>
            </thead>
            <tbody>
              {world.ledger.entries
                .slice(-25)
                .reverse()
                .map((e) => (
                  <tr key={e.entry_id}>
                    <td>{formatSimDate(e.timestamp)}</td>
                    <td>
                      <code>{e.debit_account}</code>
                    </td>
                    <td>
                      <code>{e.credit_account}</code>
                    </td>
                    <td className="right">{formatMoney(e.amount, cur)}</td>
                    <td className="small">{e.memo}</td>
                  </tr>
                ))}
            </tbody>
          </table>
        </section>
      </div>
    </div>
  );
}
