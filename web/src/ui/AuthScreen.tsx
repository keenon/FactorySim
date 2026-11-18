import { useState } from 'react';
import { useGameClient } from '../state/useGameClient';

type Mode = 'login' | 'invite' | 'reset';

export function AuthScreen() {
  const client = useGameClient();
  const [mode, setMode] = useState<Mode>('login');
  const [email, setEmail] = useState('admin@factory.local');
  const [password, setPassword] = useState('admin123');
  const [token, setToken] = useState('invite-founder-001');

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (mode === 'login') void client.login(email, password);
    else if (mode === 'invite') void client.register(token, email, password);
    else void client.resetPassword(token, password).then(() => setMode('login'));
  };

  return (
    <div className="auth-screen">
      <form className="card auth-card" onSubmit={submit}>
        <div className="brand">
          <span className="brand-mark">◆</span> Factory Sim
        </div>
        <p className="muted">Invite-only manufacturing business simulator & digital twin.</p>
        <div className="segmented">
          {(['login', 'invite', 'reset'] as const).map((m) => (
            <button type="button" key={m} className={mode === m ? 'active' : ''} onClick={() => setMode(m)}>
              {m === 'login' ? 'Sign in' : m === 'invite' ? 'Use invite' : 'Reset password'}
            </button>
          ))}
        </div>
        {client.banner && <div className="notice notice-error">{client.banner}</div>}
        {mode !== 'login' && (
          <label>
            {mode === 'invite' ? 'Invite token' : 'Reset token'}
            <input className="input" value={token} onChange={(e) => setToken(e.target.value)} />
          </label>
        )}
        {mode !== 'reset' && (
          <label>
            Email
            <input className="input" value={email} onChange={(e) => setEmail(e.target.value)} />
          </label>
        )}
        <label>
          {mode === 'reset' ? 'New password' : 'Password'}
          <input className="input" type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
        </label>
        <button className="btn btn-primary" type="submit" disabled={client.busy}>
          {mode === 'login' ? 'Sign in' : mode === 'invite' ? 'Create account' : 'Set new password'}
        </button>
        {mode === 'login' && (
          <div className="muted small">
            First-run admin: <code>admin@factory.local</code> / <code>admin123</code>
          </div>
        )}
      </form>
    </div>
  );
}
