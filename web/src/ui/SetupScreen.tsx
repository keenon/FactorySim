import { useState } from 'react';
import { formatMoney, formatSimDate } from '../logic/format';
import { useGameClient } from '../state/useGameClient';

const SCENARIOS = [
  'Conventional Starter Shop ($500k VC Seed)',
  'High-Mix Aerospace & Medical Target',
  'Container Micro-Factory Network',
];

export function SetupScreen() {
  const client = useGameClient();
  const [scenario, setScenario] = useState(SCENARIOS[0]);
  const [seed, setSeed] = useState(2026);
  const [autoPause, setAutoPause] = useState(true);
  const [resetEmail, setResetEmail] = useState('');
  const isAdmin = client.user?.role === 'admin';

  return (
    <div className="setup-screen">
      <header className="setup-header">
        <div>
          <div className="brand">
            <span className="brand-mark">◆</span> Factory Sim
          </div>
          <div className="muted">
            Signed in as <strong>{client.user?.email}</strong> · {client.user?.role}
          </div>
        </div>
        <button className="btn" onClick={() => void client.logout()}>
          Sign out everywhere
        </button>
      </header>
      {client.banner && <div className="notice notice-error">{client.banner}</div>}

      <div className="grid-3">
        <section className="card">
          <h3>New game</h3>
          <label>
            Scenario
            <select className="input" value={scenario} onChange={(e) => setScenario(e.target.value)}>
              {SCENARIOS.map((s) => (
                <option key={s}>{s}</option>
              ))}
            </select>
          </label>
          <label>
            Random seed (share it to replay the same game)
            <input className="input" type="number" value={seed} onChange={(e) => setSeed(Number(e.target.value))} />
          </label>
          <label className="check">
            <input type="checkbox" checked={autoPause} onChange={(e) => setAutoPause(e.target.checked)} /> Auto-pause on decisions and alerts
          </label>
          <button className="btn btn-primary" disabled={client.busy} onClick={() => void client.createGame(scenario, seed, autoPause)}>
            Launch new game
          </button>
        </section>

        <section className="card card-disabled">
          <h3>
            Live factory <span className="chip">offline</span>
          </h3>
          <p className="muted">Fork games and what-ifs from the real factory's estimated state once plant devices are connected.</p>
          <button className="btn" disabled>
            Connect live factory
          </button>
        </section>

        <section className="card">
          <h3>Invites & resets</h3>
          {isAdmin ? (
            <>
              <button className="btn" onClick={() => void client.createInvite()}>
                Generate player invite
              </button>
              {client.generatedInvite && (
                <div className="token">
                  Invite: <code>{client.generatedInvite}</code>
                </div>
              )}
              <div className="row">
                <input className="input" placeholder="user@factory.local" value={resetEmail} onChange={(e) => setResetEmail(e.target.value)} />
                <button className="btn" onClick={() => void client.createResetLink(resetEmail)}>
                  Reset link
                </button>
              </div>
              {client.generatedReset && (
                <div className="token">
                  Reset: <code>{client.generatedReset}</code>
                </div>
              )}
            </>
          ) : (
            <p className="muted">Ask an admin for invite and reset links.</p>
          )}
        </section>
      </div>

      <section className="card">
        <h3>Your games</h3>
        {client.games.length === 0 ? (
          <p className="muted">No games yet — launch one above to start the tutorial.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Scenario</th>
                <th>Seed</th>
                <th>Sim date</th>
                <th>Cash</th>
                <th>Runway</th>
                <th>Plants</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {client.games.map((g) => (
                <tr key={g.game_id}>
                  <td>
                    <strong>{g.scenario_name}</strong>
                  </td>
                  <td>
                    <code>{g.game_seed}</code>
                  </td>
                  <td>{formatSimDate(g.sim_time_ms)}</td>
                  <td>{formatMoney(g.cash_tenth_cents, client.currency)}</td>
                  <td>{g.runway_days_p50} d</td>
                  <td>{g.plant_count}</td>
                  <td>
                    <button className="btn btn-primary btn-sm" onClick={() => void client.openGame(g.game_id)}>
                      Resume
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
