import { formatMoney, formatSimDate, type Currency, type UnitSystem } from '../logic/format';
import { useGameClient } from '../state/useGameClient';

export function TopBar() {
  const client = useGameClient();
  const snap = client.snapshot!;
  const rw = snap.runway;
  return (
    <header className="top-bar">
      <div className="row">
        <button className="btn btn-ghost" onClick={() => void client.backToLobby()}>
          ← Games
        </button>
        <span className="brand">
          <span className="brand-mark">◆</span> Factory Sim
        </span>
        <span className="pill">
          <span className="pill-label">Clock</span>
          {formatSimDate(snap.world.now)}
        </span>
        <button className="btn" disabled={client.busy} onClick={() => void client.step(6)}>
          +6 h
        </button>
        <button className="btn btn-primary" disabled={client.busy} onClick={() => void client.skipAhead(7)} title="Space">
          Skip to next event ⏭
        </button>
      </div>
      <div className="row">
        <span className="pill">
          <span className="pill-label">Cash</span>
          <span className={rw.cash_now >= 0 ? 'pos' : 'neg'}>{formatMoney(rw.cash_now, client.currency)}</span>
        </span>
        <span className="pill">
          <span className="pill-label">Runway P10/50/90</span>
          {rw.runway_days_p10} / {rw.runway_days_p50} / {rw.runway_days_p90} d
        </span>
        <select className="input input-sm" value={client.units} onChange={(e) => client.setUnits(e.target.value as UnitSystem)}>
          <option value="metric">Metric</option>
          <option value="imperial">Imperial</option>
        </select>
        <select className="input input-sm" value={client.currency} onChange={(e) => client.setCurrency(e.target.value as Currency)}>
          <option value="USD">USD</option>
          <option value="EUR">EUR</option>
        </select>
        <button className="btn btn-ghost" onClick={() => client.setModal('problem', true)}>
          Report a problem
        </button>
      </div>
    </header>
  );
}
