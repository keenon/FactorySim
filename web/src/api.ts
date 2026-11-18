// Thin HTTP client for fsim-server. Holds no state other than the session token.

import type {
  ClientGameSnapshot,
  GameSummaryItem,
  PlantCommand,
  ProblemReportBundle,
  StaticCatalogBundle,
  UserAccount,
} from './types';

export interface AuthResponse {
  session_token: string;
  user: UserAccount;
}

export interface Api {
  setToken(token: string | null): void;
  me(): Promise<UserAccount>;
  login(email: string, password: string): Promise<AuthResponse>;
  logout(): Promise<void>;
  register(inviteToken: string, email: string, password: string): Promise<AuthResponse>;
  resetPassword(resetToken: string, newPassword: string): Promise<void>;
  createInvite(role: string): Promise<{ invite_token: string }>;
  createResetLink(email: string): Promise<{ reset_token: string }>;
  catalogs(): Promise<StaticCatalogBundle>;
  listGames(): Promise<GameSummaryItem[]>;
  createGame(scenario: string, seed: number, autoPause: boolean): Promise<ClientGameSnapshot>;
  getGame(gameId: string): Promise<ClientGameSnapshot>;
  command(gameId: string, cmd: PlantCommand): Promise<ClientGameSnapshot>;
  skip(gameId: string, horizonMs: number): Promise<ClientGameSnapshot>;
  step(gameId: string, horizonMs: number): Promise<ClientGameSnapshot>;
  validatePlacement(
    gameId: string,
    req: { plant_id: string; catalog_id: string; x_mm: number; y_mm: number; rotation_deg: number },
  ): Promise<{ valid: boolean; errors: string[] }>;
  saveCheckpoint(gameId: string, name: string): Promise<ClientGameSnapshot>;
  rewind(gameId: string, checkpointId: string): Promise<ClientGameSnapshot>;
  report(gameId: string, note: string): Promise<ProblemReportBundle>;
}

export function createHttpApi(): Api {
  let token: string | null = null;

  async function call<T>(path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (token) headers.Authorization = `Bearer ${token}`;
    const resp = await fetch(path, {
      method: body === undefined ? 'GET' : 'POST',
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const data = (await resp.json().catch(() => ({}))) as { error?: string };
    if (!resp.ok) throw new Error(data.error ?? `HTTP ${resp.status}`);
    return data as T;
  }

  const g = (id: string) => `/api/games/${encodeURIComponent(id)}`;

  return {
    setToken: (t) => {
      token = t;
    },
    me: () => call('/api/auth/me'),
    login: (email, password) => call('/api/auth/login', { email, password }),
    logout: async () => {
      await call('/api/auth/logout', {});
    },
    register: (invite_token, email, password) => call('/api/auth/register', { invite_token, email, password }),
    resetPassword: async (reset_token, new_password) => {
      await call('/api/auth/reset-password', { reset_token, new_password });
    },
    createInvite: (role) => call('/api/auth/invite', { role }),
    createResetLink: (email) => call('/api/auth/reset-link', { email }),
    catalogs: () => call('/api/catalogs'),
    listGames: () => call('/api/games'),
    createGame: (scenario_name, game_seed, auto_pause_on_alerts) =>
      call('/api/games', { scenario_name, game_seed, auto_pause_on_alerts }),
    getGame: (id) => call(g(id)),
    command: (id, cmd) => call(`${g(id)}/command`, cmd),
    skip: (id, max_horizon_ms) => call(`${g(id)}/skip`, { max_horizon_ms }),
    step: (id, max_horizon_ms) => call(`${g(id)}/step`, { max_horizon_ms }),
    validatePlacement: (id, req) => call(`${g(id)}/validate-placement`, { ...req, ignore_cell_id: null }),
    saveCheckpoint: (id, name) => call(`${g(id)}/checkpoint`, { name, is_autosave: false }),
    rewind: (id, checkpoint_id) => call(`${g(id)}/rewind`, { checkpoint_id }),
    report: (id, note) => call(`${g(id)}/report`, { note }),
  };
}
