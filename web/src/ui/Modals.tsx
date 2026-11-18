import { useState } from 'react';
import { formatMoney } from '../logic/format';
import { useGameClient } from '../state/useGameClient';

export function SkipSummaryModal() {
  const client = useGameClient();
  const s = client.snapshot?.last_skip_summary;
  if (!client.showSkipSummary || !s) return null;
  const close = () => client.setModal('skip', false);
  return (
    <div className="modal-backdrop" onClick={close}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>
          Skipped {(s.days_skipped_tenths / 10).toFixed(1)} days <span className="muted">· {s.stop_reason}</span>
        </h3>
        <div className="skip-stats">
          <div>
            <div className="muted small">Cash</div>
            {formatMoney(s.cash_start, client.currency)} → <strong>{formatMoney(s.cash_end, client.currency)}</strong>
          </div>
          <div>
            <div className="muted small">Runway (P50)</div>
            {s.runway_days_start} d → <strong>{s.runway_days_end} d</strong>
          </div>
        </div>
        <ul className="event-list">
          {s.events_occurred.map((ev, i) => (
            <li key={i}>{ev}</li>
          ))}
        </ul>
        <button className="btn btn-primary wide" onClick={close}>
          Continue
        </button>
      </div>
    </div>
  );
}

export function ProblemModal() {
  const client = useGameClient();
  const [note, setNote] = useState('');
  if (!client.showProblemModal) return null;
  const r = client.lastProblemReport;
  return (
    <div className="modal-backdrop" onClick={() => client.setModal('problem', false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>Report a problem</h3>
        <p className="muted">Saves the seed, latest checkpoint and log position so the issue replays exactly.</p>
        <input className="input" placeholder="What looks wrong?" value={note} onChange={(e) => setNote(e.target.value)} />
        {r && (
          <div className="token">
            Saved <code>{r.report_id}</code> · seed <code>{r.game_seed}</code> · log seq <code>{r.log_seq}</code>
          </div>
        )}
        <div className="row end">
          <button className="btn" onClick={() => client.setModal('problem', false)}>
            Close
          </button>
          <button className="btn btn-primary" onClick={() => void client.fileProblemReport(note || 'User report')}>
            Save report
          </button>
        </div>
      </div>
    </div>
  );
}
