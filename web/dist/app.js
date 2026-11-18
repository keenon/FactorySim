import {
  GameClient,
  formatMoney,
  formatDimension,
  project3DPoint,
  packScenePrimitives,
} from './game_client.js';

const client = new GameClient();
let activeTeamFilter = 'Machining';
let showProblemModal = false;
let lastProblemBundle = null;
let showSkipModal = false;
let authMode = 'login'; // 'login' | 'invite' | 'reset'
let generatedInviteToken = null;
let generatedResetToken = null;

async function apiFetch(path, options = {}) {
  const headers = { 'Content-Type': 'application/json', ...(options.headers || {}) };
  if (client.sessionToken) {
    headers['Authorization'] = `Bearer ${client.sessionToken}`;
  }
  const resp = await fetch(path, { ...options, headers });
  const data = await resp.json().catch(() => ({}));
  if (!resp.ok) {
    throw new Error(data.error || `HTTP ${resp.status}`);
  }
  return data;
}

async function bootstrapSession() {
  try {
    const me = await apiFetch('/api/auth/me');
    client.currentUser = me;
    await loadCatalogsAndGames();
  } catch (_) {
    client.currentUser = null;
  }
  renderApp();
}

async function loadCatalogsAndGames() {
  const [cats, games] = await Promise.all([
    apiFetch('/api/catalogs'),
    apiFetch('/api/games'),
  ]);
  client.catalogs = cats;
  client.gamesList = games;
  renderApp();
}

async function sendCommand(command) {
  if (!client.snapshot) return;
  try {
    const gid = client.snapshot.world.game_id;
    const snap = await apiFetch(`/api/games/${gid}/command`, {
      method: 'POST',
      body: JSON.stringify(command),
    });
    client.statusBanner = null;
    client.applyServerSnapshot(snap);
  } catch (err) {
    client.statusBanner = { kind: 'error', text: err.message };
    renderApp();
  }
}

async function triggerSkipAhead(days = 7) {
  if (!client.snapshot) return;
  try {
    const gid = client.snapshot.world.game_id;
    const snap = await apiFetch(`/api/games/${gid}/skip`, {
      method: 'POST',
      body: JSON.stringify({ max_horizon_ms: days * 86_400_000 }),
    });
    client.applyServerSnapshot(snap);
    showSkipModal = true;
    renderApp();
  } catch (err) {
    client.statusBanner = { kind: 'error', text: err.message };
    renderApp();
  }
}

async function triggerStepHours(hours = 6) {
  if (!client.snapshot) return;
  try {
    const gid = client.snapshot.world.game_id;
    const snap = await apiFetch(`/api/games/${gid}/step`, {
      method: 'POST',
      body: JSON.stringify({ max_horizon_ms: hours * 3_600_000 }),
    });
    client.applyServerSnapshot(snap);
  } catch (err) {
    client.statusBanner = { kind: 'error', text: err.message };
    renderApp();
  }
}

function formatSimDate(ms) {
  const days = Math.floor((ms || 0) / 86_400_000) + 1;
  const hours = Math.floor(((ms || 0) % 86_400_000) / 3_600_000);
  return `Day ${days}, ${String(hours).padStart(2, '0')}:00`;
}

// Subscribe UI rerender to `GameClient` OOP state changes
client.subscribe(() => {
  renderApp();
});

// Keyboard shortcuts (`1..6` tabs, `Space` step, `S` skip ahead, `P` planning mode)
window.addEventListener('keydown', (e) => {
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) return;
  if (!client.snapshot) return;
  if (e.key === '1') client.setTab('Plants');
  if (e.key === '2') client.setTab('Markets');
  if (e.key === '3') client.setTab('Floor');
  if (e.key === '4') client.setTab('Timeline');
  if (e.key === '5') client.setTab('Ledger');
  if (e.key === '6') client.setTab('People');
  if (e.key === 's' || e.key === 'S') triggerSkipAhead(7);
  if (e.key === 'p' || e.key === 'P') client.togglePlanningMode();
});

function renderAuthScreen() {
  return `
    <div class="auth-screen">
      <div class="auth-card">
        <h2 style="margin-bottom: 6px; color: var(--accent);">Factory Sim — Digital Twin</h2>
        <p style="color: var(--text-2); margin-bottom: 16px;">
          Invite-only manufacturing business simulator & digital twin. Nothing is reachable without signing in.
        </p>

        <div style="display:flex; gap:6px; margin-bottom: 14px;">
          <button class="btn ${authMode === 'login' ? 'btn-primary' : ''}" id="tab-auth-login">Sign In</button>
          <button class="btn ${authMode === 'invite' ? 'btn-primary' : ''}" id="tab-auth-invite">Use Invite Link</button>
          <button class="btn ${authMode === 'reset' ? 'btn-primary' : ''}" id="tab-auth-reset">Password Reset</button>
        </div>

        ${
          client.statusBanner
            ? `<div class="card" style="border-color: var(--red); color: #fca5a5;">${client.statusBanner.text}</div>`
            : ''
        }

        ${
          authMode === 'login'
            ? `
          <div style="display:flex; flex-direction:column; gap:10px;">
            <label>Email <input class="input" id="login-email" value="admin@factory.local" /></label>
            <label>Password <input class="input" id="login-password" type="password" value="admin123" /></label>
            <button class="btn btn-primary" id="btn-do-login">Sign In</button>
            <div style="color: var(--text-2); font-size: 11px;">
              Default first-run root admin: <code>admin@factory.local</code> / <code>admin123</code>
            </div>
          </div>
        `
            : authMode === 'invite'
            ? `
          <div style="display:flex; flex-direction:column; gap:10px;">
            <label>Single-Use Invite Token <input class="input" id="inv-token" value="invite-founder-001" /></label>
            <label>Email <input class="input" id="inv-email" placeholder="founder@company.com" /></label>
            <label>Password <input class="input" id="inv-password" type="password" /></label>
            <button class="btn btn-success" id="btn-do-register">Create Account with Invite</button>
          </div>
        `
            : `
          <div style="display:flex; flex-direction:column; gap:10px;">
            <label>Admin Single-Use Reset Token <input class="input" id="rst-token" placeholder="rst-..." /></label>
            <label>New Password <input class="input" id="rst-password" type="password" /></label>
            <button class="btn btn-primary" id="btn-do-reset">Set New Password</button>
          </div>
        `
        }
      </div>
    </div>
  `;
}

function renderSetupScreen() {
  const gamesRows = client.gamesList
    .map(
      (g) => `
      <tr>
        <td><strong>${g.scenario_name}</strong> <span style="color:var(--text-2)">(${g.game_id.slice(0, 12)})</span></td>
        <td><code>${g.game_seed}</code></td>
        <td>${formatSimDate(g.sim_time_ms)}</td>
        <td>${formatMoney(g.cash_tenth_cents, client.currency)}</td>
        <td>${g.runway_days_p50} days</td>
        <td>${g.plant_count}</td>
        <td><button class="btn btn-primary btn-open-game" data-gid="${g.game_id}">Resume Game</button></td>
      </tr>
    `
    )
    .join('');

  return `
    <div style="padding: 24px; max-width: 1100px; margin: 0 auto; overflow-y: auto; height: 100vh;">
      <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:20px;">
        <div>
          <h1 style="color:var(--accent); font-size:20px;">Factory Sim — Game & Twin Setup</h1>
          <div style="color:var(--text-2);">Signed in as <strong>${client.currentUser.email}</strong> (${client.currentUser.role})</div>
        </div>
        <div style="display:flex; gap:8px;">
          <button class="btn" id="btn-logout">Sign Out Everywhere</button>
        </div>
      </div>

      <div class="grid-cards" style="grid-template-columns: 1fr 1fr 1fr; margin-bottom:20px;">
        <div class="card">
          <div class="card-title">1. Start New Simulation Game</div>
          <div style="display:flex; flex-direction:column; gap:8px; margin-top:8px;">
            <label>Scenario
              <select class="input" id="new-scenario">
                <option value="Conventional Starter Shop ($500k VC Seed)">Conventional Starter Shop ($500k VC Seed)</option>
                <option value="High-Mix Aerospace & Medical Target">High-Mix Aerospace & Medical Target</option>
                <option value="Container Micro-Factory Network">Container Micro-Factory Network</option>
              </select>
            </label>
            <label>Deterministic Random Seed (editable for sharing & replay)
              <input class="input" id="new-seed" type="number" value="2026" />
            </label>
            <label style="display:flex; align-items:center; gap:6px;">
              <input type="checkbox" id="new-autopause" checked /> Auto-pause on decision inbox & alerts
            </label>
            <button class="btn btn-success" id="btn-create-game">Launch New Game</button>
          </div>
        </div>

        <div class="card" style="opacity:0.62;">
          <div class="card-title">2. Connect to Live Factory Twin <span class="badge badge-idle">Offline</span></div>
          <p style="color:var(--text-2); margin: 10px 0;">
            Fork a game or what-if branch directly from the real factory's current estimated state snapshot and mTLS device stream.
          </p>
          <button class="btn" disabled>Live Factory Twin (Disabled until plant hardware connected)</button>
        </div>

        <div class="card">
          <div class="card-title">3. Admin & Invite Management</div>
          <p style="color:var(--text-2); margin-bottom:8px;">Generate single-use invite or password reset links.</p>
          <div style="display:flex; gap:6px; margin-bottom:8px;">
            <button class="btn" id="btn-gen-invite">Generate Player Invite Link</button>
          </div>
          ${
            generatedInviteToken
              ? `<div style="padding:6px; background:var(--bg-0); border-radius:4px; margin-bottom:8px;">New Invite Token: <code>${generatedInviteToken}</code></div>`
              : ''
          }
          <div style="display:flex; gap:6px;">
            <input class="input" id="rst-target-email" placeholder="user@factory.local" />
            <button class="btn" id="btn-gen-reset">Reset Link</button>
          </div>
          ${
            generatedResetToken
              ? `<div style="padding:6px; background:var(--bg-0); border-radius:4px; margin-top:6px;">Reset Token: <code>${generatedResetToken}</code></div>`
              : ''
          }
        </div>
      </div>

      <div class="card">
        <div class="card-title">Saved Games & Checkpoints</div>
        ${
          client.gamesList.length === 0
            ? `<p style="color:var(--text-2); padding: 12px 0;">No games yet. Click <strong>Launch New Game</strong> above to start the interactive tutorial or sandbox!</p>`
            : `
          <table class="data-table">
            <thead>
              <tr><th>Scenario</th><th>Seed</th><th>Sim Date</th><th>Cash</th><th>Runway (P50)</th><th>Plants</th><th>Action</th></tr>
            </thead>
            <tbody>${gamesRows}</tbody>
          </table>
        `
        }
      </div>
    </div>
  `;
}

function renderTutorialBanner(w) {
  if (!w.tutorial_step || w.tutorial_step === 0) return '';
  const steps = {
    1: 'Step 1 of 5: Lease your first plant on the Plants tab (e.g., Suburban Light-Industrial Bay).',
    2: 'Step 2 of 5: Open the Floor tab and place 1 CNC Cell (Machining) and 1 Manual Pack Station (Finishing & Packing).',
    3: 'Step 3 of 5: Open the People tab and hire 1 operator (they carry an iPad for automatic task dispatch).',
    4: 'Step 4 of 5: Open the Markets tab and enable the Online Prototyping Marketplace with the middle price policy.',
    5: 'Step 5 of 5: Click "Skip Ahead to Next Event" in the top bar to watch keys arrive, machines commission, orders chain backward, and revenue land!',
  };
  return `
    <div class="tutorial-banner">
      <div>
        <strong style="color:var(--accent);">Interactive Tutorial:</strong>
        <span style="margin-left:8px;">${steps[w.tutorial_step] || ''}</span>
      </div>
      <div style="display:flex; gap:8px;">
        <button class="btn btn-primary" id="btn-tutorial-auto-step">Do This Step For Me</button>
        <button class="btn" id="btn-tutorial-skip">Skip Tutorial (Sandbox)</button>
      </div>
    </div>
  `;
}

function renderPlantsTab(w) {
  const leasedCards = Object.values(w.plants).map((p) => {
    const ready = p.keys_received && p.utilities_ready;
    return `
      <div class="card" style="border-color:${client.selectedPlantId === p.plant_id ? 'var(--accent)' : 'var(--border)'}">
        <div class="card-title">
          <span>${p.name} (<code>${p.plant_id}</code>)</span>
          <span class="badge ${ready ? 'badge-running' : 'badge-installing'}">${ready ? 'Operational' : 'In Build-Out Pipeline'}</span>
        </div>
        <div style="color:var(--text-1); margin-bottom:8px;">
          Keys: <strong>${p.keys_received ? 'Received' : `ETA ${formatSimDate(p.keys_ready_at)}`}</strong> ·
          480V Utilities: <strong>${p.utilities_ready ? 'Ready' : `ETA ${formatSimDate(p.utilities_ready_at)}`}</strong>
        </div>
        <button class="btn btn-primary btn-select-plant" data-pid="${p.plant_id}">
          ${client.selectedPlantId === p.plant_id ? 'Active Selected Plant' : 'Switch View to This Plant'}
        </button>
      </div>
    `;
  });

  const listingCards = client.catalogs.buildings.map((b) => {
    const bbox = b.floor_polygon.outer;
    const widthMm = bbox[1]?.x || 20000;
    const depthMm = bbox[2]?.y || 15000;
    return `
      <div class="card">
        <div class="card-title">
          <span>${b.name}</span>
          <span class="badge badge-idle">${b.building_kind.toUpperCase()}</span>
        </div>
        <p style="color:var(--text-2); margin-bottom:8px;">${b.description}</p>
        <div style="font-size:12px; color:var(--text-1); margin-bottom:10px; line-height:1.5;">
          <div>Floor: <strong>${formatDimension(widthMm, client.unitSystem)} × ${formatDimension(depthMm, client.unitSystem)}</strong> · Clear Height: <strong>${formatDimension(b.clear_height_mm, client.unitSystem)}</strong></div>
          <div>Max Door: <strong>${formatDimension(b.max_door_width_mm, client.unitSystem)} × ${formatDimension(b.max_door_height_mm, client.unitSystem)}</strong> · Power: <strong>${b.power_capacity_kw} kW (480V 3φ)</strong></div>
          <div>Rent: <strong>${formatMoney(b.monthly_rent, client.currency)}/mo</strong> · Deposit: <strong>${formatMoney(b.lease_deposit, client.currency)}</strong></div>
          <div>Lead Time: Keys ${b.keys_lead_days_min}–${b.keys_lead_days_max}d, Utilities ${b.utilities_lead_days_min}–${b.utilities_lead_days_max}d</div>
        </div>
        <button class="btn btn-success btn-lease-building" data-lid="${b.listing_id}" data-bname="${b.name}">
          Sign Lease & Start Utility Hookup
        </button>
      </div>
    `;
  });

  return `
    <div>
      <h3 style="margin-bottom:10px;">Your Leased Plants (${leasedCards.length})</h3>
      ${
        leasedCards.length === 0
          ? `<div class="card" style="border-color:var(--amber);">You have no leased plants yet. Choose a site below to sign a lease and start your factory!</div>`
          : `<div class="grid-cards" style="margin-bottom:18px;">${leasedCards.join('')}</div>`
      }
      <h3 style="margin-bottom:10px;">Available Site Listings (Warehouses, Urban Lofts, Container Units)</h3>
      <div class="grid-cards">${listingCards.join('')}</div>
    </div>
  `;
}

function renderMarketsTab(w) {
  if (!client.selectedPlantId) {
    return `<div class="card">Lease a plant on the <strong>Plants</strong> tab first to configure its markets.</div>`;
  }
  const policyMap = new Map(w.market_policies || []);
  const mktCards = client.catalogs.markets.map((m) => {
    const key = policyMap.get ? null : null;
    let pol = null;
    for (const [[pid, mid], p] of w.market_policies || []) {
      if (pid === client.selectedPlantId && mid === m.market_id) {
        pol = p;
      }
    }
    if (!pol) {
      pol = {
        market_id: m.market_id,
        enabled: false,
        min_payout_per_hour: 80000,
        price_multiplier_pct: 100,
        promised_lead_days: 7,
      };
    }
    const missingCerts = m.required_certs.filter((c) => !(w.unlocked_certs || []).includes(c));
    const rep = (w.market_reputation_permille && w.market_reputation_permille[m.market_id]) || 650;

    return `
      <div class="card" style="border-color:${pol.enabled ? 'var(--green)' : 'var(--border)'}">
        <div class="card-title">
          <span>${m.name}</span>
          <span class="badge ${pol.enabled ? 'badge-running' : 'badge-idle'}">${pol.enabled ? 'ACTIVE' : 'OFF'}</span>
        </div>
        <p style="color:var(--text-2); margin-bottom:8px;">${m.description}</p>
        <div style="font-size:12px; margin-bottom:8px;">
          Ref Payout: <strong>${formatMoney(m.reference_payout_per_part, client.currency)}/part</strong> ·
          Market Reputation: <strong>${(rep / 10).toFixed(1)}%</strong>
        </div>
        ${
          missingCerts.length > 0
            ? `<div style="color:var(--amber); margin-bottom:8px;">Requires Certifications: ${missingCerts.join(', ')}</div>`
            : ''
        }
        <div style="display:grid; grid-template-columns: 1fr 1fr; gap:8px; margin-bottom:10px;">
          <label>Price Policy (% of ref)
            <input class="input mkt-price-${m.market_id}" type="number" value="${pol.price_multiplier_pct}" />
          </label>
          <label>Promised Lead Days
            <input class="input mkt-lead-${m.market_id}" type="number" value="${pol.promised_lead_days}" />
          </label>
        </div>
        <button class="btn ${pol.enabled ? 'btn-warning' : 'btn-success'} btn-toggle-market"
          data-mid="${m.market_id}" data-enable="${!pol.enabled}" ${missingCerts.length > 0 ? 'disabled' : ''}>
          ${pol.enabled ? 'Pause Order Acceptance' : 'Turn On Market & Accept Orders'}
        </button>
      </div>
    `;
  });

  const certCards = client.catalogs.certifications.map((c) => {
    const unlocked = (w.unlocked_certs || []).includes(c.cert_id);
    return `
      <div class="card">
        <div class="card-title">
          <span>${c.name}</span>
          <span class="badge ${unlocked ? 'badge-running' : 'badge-idle'}">${unlocked ? 'Registered' : c.company_wide ? 'Company-Wide' : 'Site Cert'}</span>
        </div>
        <p style="color:var(--text-2); margin-bottom:8px;">${c.description}</p>
        <div style="margin-bottom:8px;">Audit Cost: <strong>${formatMoney(c.cost, client.currency)}</strong> · Lead Time: <strong>${c.lead_days} days</strong></div>
        <button class="btn btn-primary btn-unlock-cert" data-cid="${c.cert_id}" ${unlocked ? 'disabled' : ''}>
          ${unlocked ? 'Unlocked' : 'Start Audit & Unlock Certification'}
        </button>
      </div>
    `;
  });

  const orderRows = Object.values(w.customer_orders || {})
    .slice(-15)
    .reverse()
    .map(
      (o) => `
      <tr>
        <td><code>${o.customer_order_id}</code></td>
        <td>${o.part_name}</td>
        <td><code>${o.material_sku}</code></td>
        <td>${formatMoney(o.payout, client.currency)}</td>
        <td>${formatSimDate(o.due_at_ms)}</td>
        <td><span class="badge ${o.status === 'active' ? 'badge-installing' : 'badge-running'}">${o.status}</span></td>
      </tr>
    `
    )
    .join('');

  return `
    <div>
      <h3 style="margin-bottom:10px;">Markets & Bidding Policies (${client.selectedPlantId})</h3>
      <div class="grid-cards" style="margin-bottom:16px;">${mktCards.join('')}</div>

      <h3 style="margin-bottom:10px;">Active & Recent Customer Orders (${Object.keys(w.customer_orders || {}).length})</h3>
      <div class="card" style="margin-bottom:16px;">
        ${
          orderRows.length === 0
            ? `<p style="color:var(--text-2);">No orders yet. Turn on the Online Prototyping Marketplace above and Skip Ahead!</p>`
            : `<table class="data-table"><thead><tr><th>Order ID</th><th>Part</th><th>Input Stock</th><th>Payout</th><th>Due</th><th>Status</th></tr></thead><tbody>${orderRows}</tbody></table>`
        }
      </div>

      <h3 style="margin-bottom:10px;">Certifications (Site & Company-Wide)</h3>
      <div class="grid-cards">${certCards.join('')}</div>
    </div>
  `;
}

function renderFloorTab(w) {
  if (!client.selectedPlantId) {
    return `<div class="card">Lease a plant on the <strong>Plants</strong> tab first to view and lay out its floor.</div>`;
  }
  const plant = w.plants[client.selectedPlantId];
  const listing = client.catalogs.buildings.find((b) => b.listing_id === plant.listing_id);

  const teamNames = [
    'Machining',
    'FinishingPacking',
    'MaterialsLogistics',
    'Quality',
    'Molding',
    'Toolroom',
    'HeatTreat',
    'FacilitiesEhs',
  ];

  const teamButtons = teamNames
    .map(
      (t) => `
      <button class="btn ${activeTeamFilter === t ? 'btn-primary' : ''} btn-team-filter" data-team="${t}" style="padding:4px 7px; font-size:11px;">
        ${t}
      </button>
    `
    )
    .join('');

  const filteredCells = client.catalogs.cells.filter((c) => c.team === activeTeamFilter);
  const paletteItems = filteredCells
    .map((c) => {
      let disabledReason = null;
      if (listing && c.height_mm > listing.clear_height_mm) {
        disabledReason = `Needs ${formatDimension(c.height_mm, client.unitSystem)} clear height`;
      } else if (c.needs_climate_room && !plant.climate_room_built) {
        disabledReason = 'Needs Modular CMM Climate Room';
      } else if (c.needs_hazmat_permit && !plant.hazmat_permit_approved) {
        disabledReason = 'Needs Hazmat Permit';
      }
      return `
        <div class="card" style="padding:9px; margin-bottom:8px;">
          <div style="font-weight:700; color:var(--accent);">${c.display_name}</div>
          <div style="font-size:11px; color:var(--text-2); margin:3px 0;">${c.description}</div>
          <div style="font-size:11px; margin-bottom:6px;">
            <strong>${formatMoney(c.price, client.currency)}</strong> ·
            ${formatDimension(c.width_mm, client.unitSystem)}×${formatDimension(c.depth_mm, client.unitSystem)} ·
            Lead ${c.lead_time_days_min}–${c.lead_time_days_max}d
            ${c.financeable ? '· <span style="color:var(--green)">Financeable</span>' : '· <span style="color:var(--amber)">Cash Only</span>'}
          </div>
          <div style="font-size:11px; color:var(--text-1); margin-bottom:6px;">${c.cycle_time_summary}</div>
          ${
            disabledReason
              ? `<button class="btn" disabled style="width:100%;">Unavailable: ${disabledReason}</button>`
              : `<button class="btn btn-primary btn-place-catalog-cell" data-cat="${c.catalog_id}" style="width:100%;">
                  ${client.planningMode ? 'Add Ghost to Plan' : 'Place & Order Cell'}
                </button>`
          }
        </div>
      `;
    })
    .join('');

  return `
    <div class="floor-layout">
      <div class="palette-sidebar">
        <div style="font-weight:700; margin-bottom:6px;">Build Palette by Team</div>
        <div style="display:flex; flex-wrap:wrap; gap:4px; margin-bottom:10px;">${teamButtons}</div>
        <div>${paletteItems}</div>
      </div>

      <div class="canvas-wrap">
        <div style="display:flex; justify-content:space-between; align-items:center; padding:8px 12px; background:var(--bg-1); border-bottom:1px solid var(--border);">
          <div style="display:flex; align-items:center; gap:8px;">
            <strong>${plant.name}</strong>
            <span style="color:var(--text-2); font-size:11px;">(Click any cell or square to inspect "Why isn't this running?")</span>
          </div>
          <div style="display:flex; gap:6px; align-items:center;">
            <button class="btn ${client.planningMode ? 'btn-warning' : ''}" id="btn-toggle-planning">
              ${client.planningMode ? 'Planning Mode: ON (Ghosts)' : 'Planning Mode: OFF'}
            </button>
            ${
              client.planningMode
                ? `
                <span style="font-size:11px;">Plan Cost: <strong>${formatMoney(client.plannedTotalCostTenthCents(), client.currency)}</strong> (${client.plannedCells.length} ghosts)</span>
                <button class="btn" id="btn-plan-undo" ${client.undoStack.length === 0 ? 'disabled' : ''}>Undo</button>
                <button class="btn btn-success" id="btn-plan-execute" ${client.plannedCells.length === 0 ? 'disabled' : ''}>Commit & Execute Plan</button>
                <button class="btn" id="btn-plan-clear" ${client.plannedCells.length === 0 ? 'disabled' : ''}>Clear</button>
              `
                : ''
            }
            <button class="btn" id="btn-order-stock">+ Order Raw Stock & Boxes</button>
          </div>
        </div>
        <canvas id="floor-canvas" width="900" height="560"></canvas>
      </div>
    </div>
  `;
}

function renderTimelineTab(w) {
  const pipes = Object.values(w.pipeline || {});
  const orders = Object.values(w.internal_orders || {});

  const pipeRows = pipes
    .map((p) => {
      const spanMs = Math.max(86_400_000, p.eta_p90 - p.started_at);
      const p10Pct = Math.min(85, Math.max(5, ((p.eta_p10 - p.started_at) / spanMs) * 75));
      const p90Pct = Math.min(98, Math.max(p10Pct + 10, ((p.eta_p90 - p.started_at) / spanMs) * 90));
      return `
        <div class="waterfall-row">
          <div>
            <div style="font-weight:600;">${p.title}</div>
            <div style="font-size:11px; color:var(--text-2);">${p.category} · Cost ${formatMoney(p.cost, client.currency)}</div>
          </div>
          <div class="waterfall-track">
            <div class="waterfall-band" style="left:${p10Pct}%; width:${p90Pct - p10Pct}%; background:${p.completed ? 'rgba(34,197,94,0.35)' : 'rgba(56,189,248,0.3)'}"></div>
          </div>
          <div style="text-align:right;">
            <span class="badge ${p.completed ? 'badge-running' : 'badge-installing'}">
              ${p.completed ? 'Done' : `P50: ${formatSimDate(p.eta_p50)}`}
            </span>
          </div>
        </div>
      `;
    })
    .join('');

  const orderRows = orders
    .slice(-20)
    .reverse()
    .map(
      (o) => `
      <tr>
        <td><code>${o.order_id}</code></td>
        <td><code>${o.parent_order_id || 'ROOT (Shipping)'}</code></td>
        <td>${o.target_team}</td>
        <td><code>${o.target_cell_id || 'unassigned'}</code></td>
        <td><code>${o.output_spec.item_sku}</code></td>
        <td><span class="badge ${o.status === 'Completed' ? 'badge-running' : o.status === 'BlockedUpstream' ? 'badge-blocked' : 'badge-installing'}">${o.status}</span></td>
        <td>${(o.upstream_order_ids || []).join(', ') || 'Leaf'}</td>
      </tr>
    `
    )
    .join('');

  return `
    <div>
      <div class="card">
        <div class="card-title">Pipeline Waterfall (Commitments in Flight with P10–P90 Uncertainty Bands)</div>
        ${
          pipeRows.length === 0
            ? `<p style="color:var(--text-2);">No pipeline items yet.</p>`
            : pipeRows
        }
      </div>

      <div class="card">
        <div class="card-title">Backward-Chaining Pull Orders (Shipping → Packing → CNC → Saw → Stock)</div>
        ${
          orderRows.length === 0
            ? `<p style="color:var(--text-2);">No internal pull orders yet.</p>`
            : `
          <table class="data-table">
            <thead>
              <tr><th>Order ID</th><th>Unblocks Parent</th><th>Team</th><th>Cell</th><th>Output SKU</th><th>Status</th><th>Upstream Dependencies</th></tr>
            </thead>
            <tbody>${orderRows}</tbody>
          </table>
        `
        }
      </div>
    </div>
  `;
}

function renderLedgerTab(w, snap) {
  const rw = snap.runway;
  const bn = snap.bottleneck;
  const balances = Object.entries(w.ledger.account_balances || {})
    .map(
      ([acct, val]) => `
      <tr>
        <td><code>${acct}</code></td>
        <td style="font-weight:700;">${formatMoney(val, client.currency)}</td>
      </tr>
    `
    )
    .join('');

  const entries = (w.ledger.entries || [])
    .slice(-20)
    .reverse()
    .map(
      (e) => `
      <tr>
        <td>${formatSimDate(e.timestamp)}</td>
        <td><code>${e.debit_account}</code></td>
        <td><code>${e.credit_account}</code></td>
        <td><strong>${formatMoney(e.amount, client.currency)}</strong></td>
        <td>${e.memo}</td>
      </tr>
    `
    )
    .join('');

  return `
    <div>
      <div class="grid-cards" style="grid-template-columns: 1fr 1fr 1fr; margin-bottom:14px;">
        <div class="card">
          <div class="card-title">Cash & Runway Fan Projection</div>
          <div style="font-size:18px; font-weight:800; color:var(--green); margin:6px 0;">
            Cash: ${formatMoney(rw.cash_now, client.currency)}
          </div>
          <div style="font-size:12px; line-height:1.6;">
            <div>Credit Line Headroom: <strong>${formatMoney(rw.credit_available, client.currency)}</strong></div>
            <div>Gross Monthly Burn: <strong>${formatMoney(rw.gross_monthly_burn, client.currency)}/mo</strong></div>
            <div>Runway (P10 / P50 / P90): <strong>${rw.runway_days_p10}d / ${rw.runway_days_p50}d / ${rw.runway_days_p90}d</strong></div>
          </div>
        </div>

        <div class="card">
          <div class="card-title">Bottleneck & Shadow Prices</div>
          <div style="font-size:13px; color:var(--accent); font-weight:700; margin:6px 0;">
            ${bn.recommendation}
          </div>
          <div style="font-size:12px; color:var(--text-1); line-height:1.5;">
            <div>Estimated Utilization: <strong>${(bn.utilization_permille / 10).toFixed(1)}%</strong></div>
            <div>Lagrange Shadow Value: <strong>$${bn.shadow_price_dollars_per_hr}/hr</strong></div>
            <div>Gateway Hash-Chained Log Entries: <strong>${(w.gateway.log_entries || []).length}</strong></div>
          </div>
        </div>

        <div class="card">
          <div class="card-title">Equipment Financing (8.5% APR, 36m)</div>
          <p style="color:var(--text-2); margin-bottom:8px;">Borrow up to 80% against resellable CNC/Molding machines on the floor.</p>
          <button class="btn btn-primary" id="btn-take-loan">Borrow $45,000 Against Installed Machine</button>
        </div>
      </div>

      <div class="grid-cards" style="grid-template-columns: 340px 1fr;">
        <div class="card">
          <div class="card-title">Double-Entry Account Balances (Sum = 0)</div>
          <table class="data-table">
            <thead><tr><th>Account</th><th>Signed Balance</th></tr></thead>
            <tbody>${balances}</tbody>
          </table>
        </div>

        <div class="card">
          <div class="card-title">Recent Double-Entry Journal Entries</div>
          <table class="data-table">
            <thead><tr><th>Date</th><th>Debit</th><th>Credit</th><th>Amount</th><th>Memo</th></tr></thead>
            <tbody>${entries}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
}

function renderPeopleTab(w) {
  if (!client.selectedPlantId) {
    return `<div class="card">Lease a plant on the <strong>Plants</strong> tab first to hire and assign workers.</div>`;
  }
  const workerRows = Object.values(w.workers || {})
    .map((wk) => {
      const onboarded = w.now >= wk.ready_at;
      const hrs = ((wk.worked_ms_in_domain || 0) / 3_600_000).toFixed(1);
      const rampPct = Math.round((1.0 - 0.75 * Math.exp(-hrs / 80.0)) * 100);
      return `
        <tr>
          <td><strong>${wk.name}</strong> (<code>${wk.worker_id}</code>)</td>
          <td>${wk.role}</td>
          <td><code>${wk.ipad_device_id}</code></td>
          <td>${formatMoney(wk.wage_per_hour, client.currency)}/hr</td>
          <td>${rampPct}% (${hrs}h domain exp)</td>
          <td>
            <span class="badge ${!onboarded ? 'badge-installing' : wk.active_task ? 'badge-running' : 'badge-idle'}">
              ${!onboarded ? `Onboarding (Ready ${formatSimDate(wk.ready_at)})` : wk.active_task ? `Executing ${wk.active_task.task.category} (${wk.active_task.task.order_id})` : 'Idle (Waiting on iPad)'}
            </span>
          </td>
        </tr>
      `;
    })
    .join('');

  return `
    <div>
      <div class="card" style="display:flex; justify-content:space-between; align-items:center;">
        <div>
          <div class="card-title">Hire Plant Operator (Equips Worker with Dispatched iPad on fsim-net)</div>
          <div style="color:var(--text-2);">Workers carry iPads, cannot be interrupted mid-task, and ramp up productivity as 1 − e^(−t/τ).</div>
        </div>
        <button class="btn btn-success" id="btn-hire-operator">+ Hire Operator ($32/hr)</button>
      </div>

      <div class="card">
        <div class="card-title">Company Roster & Live iPad Task Status (${Object.keys(w.workers || {}).length})</div>
        ${
          workerRows.length === 0
            ? `<p style="color:var(--text-2);">No employees hired yet. Click <strong>+ Hire Operator</strong> above!</p>`
            : `<table class="data-table"><thead><tr><th>Worker</th><th>Role</th><th>iPad Device ID</th><th>Wage</th><th>Ramp-Up</th><th>Current iPad Task</th></tr></thead><tbody>${workerRows}</tbody></table>`
        }
      </div>

      <div class="card">
        <div class="card-title">Central iPad Task Dispatch Priority Order</div>
        <p style="color:var(--text-2); margin-bottom:8px;">Order in which the People module dispatches unblocked floor tasks to idle worker iPads:</p>
        <div style="display:flex; gap:8px; flex-wrap:wrap;">
          ${(w.shift_priorities || []).map((p, i) => `<span class="metric-pill"><strong>#${i + 1}</strong> ${p}</span>`).join('')}
        </div>
      </div>
    </div>
  `;
}

function renderRightDetailInspector(w, snap) {
  const statuses = snap.cell_statuses || [];
  const selectedCell = w.cells && w.cells[client.selectedEntityId];
  const selectedStatus = statuses.find((s) => s.cell_id === client.selectedEntityId);

  const cellStatusCards = statuses
    .map(
      (st) => `
      <div class="card" style="padding:9px; margin-bottom:8px; cursor:pointer; border-color:${client.selectedEntityId === st.cell_id ? 'var(--accent)' : 'var(--border)'}" data-select-cell="${st.cell_id}">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:4px;">
          <strong><code>${st.cell_id}</code></strong>
          <span class="badge ${st.state === 'Running' ? 'badge-running' : st.state === 'Installing' ? 'badge-installing' : st.state === 'Blocked' ? 'badge-blocked' : 'badge-idle'}">${st.state}</span>
        </div>
        <div style="font-size:12px; color:var(--text-0); margin-bottom:4px;">${st.message}</div>
        <div style="font-size:11px; color:var(--text-2);">
          Reason: <code>${st.reason}</code>
          ${st.blocker_link ? ` · Blocker: <code>${st.blocker_link}</code>` : ''}
        </div>
      </div>
    `
    )
    .join('');

  return `
    <div>
      <div class="card-title" style="border-bottom:1px solid var(--border); padding-bottom:8px;">
        <span>Inspector & "Why Isn't This Running?"</span>
      </div>

      ${
        selectedCell
          ? `
        <div class="card" style="border-color:var(--accent); margin-top:10px;">
          <div class="card-title">${selectedCell.display_name}</div>
          <div style="font-size:12px; line-height:1.6;">
            <div>Cell ID: <code>${selectedCell.cell_id}</code> · Team: <strong>${selectedCell.team}</strong></div>
            <div>Device Session: <code>${selectedCell.device_id}</code></div>
            <div>Position: (${formatDimension(selectedCell.x_mm, client.unitSystem)}, ${formatDimension(selectedCell.y_mm, client.unitSystem)})</div>
            <div>Cycles Completed: <strong>${selectedCell.cycles_completed}</strong></div>
            <div>Tool Wear: <strong>${selectedCell.tool_wear_pct}%</strong> · Chip Hopper: <strong>${selectedCell.chip_hopper_pct}%</strong></div>
            ${
              selectedStatus
                ? `<div style="margin-top:6px; padding:6px; background:var(--bg-0); border-radius:4px;">
                    <strong>Status:</strong> ${selectedStatus.message}<br/>
                    <span style="color:var(--text-2); font-size:11px;">${selectedStatus.uncertainty_note || ''}</span>
                  </div>`
                : ''
            }
          </div>
        </div>
      `
          : ''
      }

      <div style="margin-top:10px;">
        <div style="font-weight:700; margin-bottom:6px; color:var(--text-1);">All Floor Cells — Live Status (${statuses.length})</div>
        ${
          cellStatusCards.length === 0
            ? `<div style="color:var(--text-2);">No cells placed yet.</div>`
            : cellStatusCards
        }
      </div>

      <div class="card" style="margin-top:12px;">
        <div class="card-title">Checkpoints & Rewind (${(snap.checkpoints || []).length})</div>
        <div style="display:flex; gap:6px; margin-bottom:8px;">
          <input class="input" id="ck-name-input" placeholder="Checkpoint name..." />
          <button class="btn btn-primary" id="btn-save-checkpoint">Save</button>
        </div>
        <div style="max-height:160px; overflow-y:auto;">
          ${(snap.checkpoints || [])
            .slice()
            .reverse()
            .map(
              (ck) => `
              <div style="display:flex; justify-content:space-between; align-items:center; padding:5px 0; border-bottom:1px solid var(--border); font-size:11px;">
                <div>
                  <strong>${ck.name}</strong><br/>
                  <span style="color:var(--text-2)">${formatSimDate(ck.sim_time_ms)} · ${formatMoney(ck.cash_tenth_cents, client.currency)}</span>
                </div>
                <button class="btn btn-rewind-ck" data-ckid="${ck.checkpoint_id}">Rewind</button>
              </div>
            `
            )
            .join('')}
        </div>
      </div>
    </div>
  `;
}

function renderSkipModal() {
  if (!showSkipModal || !client.snapshot?.last_skip_summary) return '';
  const s = client.snapshot.last_skip_summary;
  const days = (s.days_skipped_tenths / 10).toFixed(1);
  return `
    <div class="modal-backdrop">
      <div class="modal-box">
        <div class="card-title" style="color:var(--accent); font-size:16px; margin-bottom:10px;">
          Skipped ${days} Days (${s.stop_reason})
        </div>
        <div style="font-size:14px; line-height:1.8; margin-bottom:12px;">
          <div>Cash: <strong>${formatMoney(s.cash_start, client.currency)}</strong> → <strong style="color:var(--green)">${formatMoney(s.cash_end, client.currency)}</strong></div>
          <div>Runway (P50): <strong>${s.runway_days_start} days</strong> → <strong>${s.runway_days_end} days</strong></div>
        </div>
        <div style="background:var(--bg-0); padding:10px; border-radius:6px; margin-bottom:14px;">
          <div style="font-weight:700; margin-bottom:6px;">Events Along the Skipped Stretch:</div>
          <ul style="padding-left:18px; line-height:1.6;">
            ${(s.events_occurred || []).map((ev) => `<li>${ev}</li>`).join('')}
          </ul>
        </div>
        <button class="btn btn-primary" id="btn-close-skip-modal" style="width:100%;">Continue Playing</button>
      </div>
    </div>
  `;
}

function renderProblemModal() {
  if (!showProblemModal) return '';
  return `
    <div class="modal-backdrop">
      <div class="modal-box">
        <div class="card-title">Report a Problem (Deterministic Replay Bundle)</div>
        <p style="color:var(--text-2); margin-bottom:10px;">
          Captures the game's seed, current checkpoint, and gateway log sequence position so any issue reproduces bit-identically.
        </p>
        <input class="input" id="problem-note-input" placeholder="Describe what looks wrong..." style="margin-bottom:10px;" />
        ${
          lastProblemBundle
            ? `<div style="background:var(--bg-0); padding:10px; border-radius:6px; margin-bottom:10px; font-size:11px;">
                Saved Report <code>${lastProblemBundle.report_id}</code> ·
                Seed: <code>${lastProblemBundle.game_seed}</code> ·
                Log Seq: <code>${lastProblemBundle.log_seq}</code> ·
                Checkpoint: <code>${lastProblemBundle.latest_checkpoint_id || 'none'}</code>
              </div>`
            : ''
        }
        <div style="display:flex; gap:8px; justify-content:flex-end;">
          <button class="btn btn-primary" id="btn-submit-problem">Save Diagnostic Bundle</button>
          <button class="btn" id="btn-close-problem-modal">Close</button>
        </div>
      </div>
    </div>
  `;
}

function drawFloor3DCanvas() {
  const canvas = document.getElementById('floor-canvas');
  if (!canvas || !client.snapshot || !client.selectedPlantId) return;
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);

  const plant = client.snapshot.world.plants[client.selectedPlantId];
  if (!plant) return;
  const listing = client.catalogs.buildings.find((b) => b.listing_id === plant.listing_id);
  const cam = client.camera;

  // 1. Draw building floor polygon & grid
  if (listing && listing.floor_polygon?.outer) {
    ctx.save();
    ctx.beginPath();
    listing.floor_polygon.outer.forEach((pt, idx) => {
      const p = project3DPoint(pt.x, pt.y, 0, cam);
      if (idx === 0) ctx.moveTo(p.sx, p.sy);
      else ctx.lineTo(p.sx, p.sy);
    });
    ctx.closePath();
    ctx.fillStyle = '#111827';
    ctx.fill();
    ctx.lineWidth = 2;
    ctx.strokeStyle = '#475569';
    ctx.stroke();

    // Draw interior structural column holes if any
    for (const hole of listing.floor_polygon.holes || []) {
      ctx.beginPath();
      hole.forEach((pt, idx) => {
        const p = project3DPoint(pt.x, pt.y, 0, cam);
        if (idx === 0) ctx.moveTo(p.sx, p.sy);
        else ctx.lineTo(p.sx, p.sy);
      });
      ctx.closePath();
      ctx.fillStyle = '#334155';
      ctx.fill();
      ctx.strokeStyle = '#ef4444';
      ctx.stroke();
    }
    ctx.restore();
  }

  // 2. Pack and draw extruded 3D boxes for cells, squares, planning ghosts, and workers
  const primitives = packScenePrimitives(
    client.snapshot,
    client.selectedPlantId,
    client.plannedCells,
    client.snapshot.world.now
  );

  for (const prim of primitives) {
    const b0 = project3DPoint(prim.x_mm, prim.y_mm, 0, cam);
    const b1 = project3DPoint(prim.x_mm + prim.width_mm, prim.y_mm, 0, cam);
    const b2 = project3DPoint(prim.x_mm + prim.width_mm, prim.y_mm + prim.depth_mm, 0, cam);
    const b3 = project3DPoint(prim.x_mm, prim.y_mm + prim.depth_mm, 0, cam);

    const t0 = project3DPoint(prim.x_mm, prim.y_mm, prim.height_mm, cam);
    const t1 = project3DPoint(prim.x_mm + prim.width_mm, prim.y_mm, prim.height_mm, cam);
    const t2 = project3DPoint(prim.x_mm + prim.width_mm, prim.y_mm + prim.depth_mm, prim.height_mm, cam);
    const t3 = project3DPoint(prim.x_mm, prim.y_mm + prim.depth_mm, prim.height_mm, cam);

    ctx.save();
    ctx.globalAlpha = prim.ghost ? 0.52 : 0.92;

    // Front extruded face
    ctx.beginPath();
    ctx.moveTo(b3.sx, b3.sy);
    ctx.lineTo(b2.sx, b2.sy);
    ctx.lineTo(t2.sx, t2.sy);
    ctx.lineTo(t3.sx, t3.sy);
    ctx.closePath();
    ctx.fillStyle = prim.color;
    ctx.fill();
    ctx.strokeStyle = '#0f172a';
    ctx.stroke();

    // Top extruded roof face
    ctx.beginPath();
    ctx.moveTo(t0.sx, t0.sy);
    ctx.lineTo(t1.sx, t1.sy);
    ctx.lineTo(t2.sx, t2.sy);
    ctx.lineTo(t3.sx, t3.sy);
    ctx.closePath();
    ctx.fillStyle = prim.color;
    ctx.fill();
    ctx.lineWidth = prim.ghost ? 2 : 1.2;
    ctx.strokeStyle = prim.ghost ? '#38bdf8' : '#e2e8f0';
    ctx.stroke();

    // Label
    ctx.globalAlpha = 1.0;
    ctx.fillStyle = '#f8fafc';
    ctx.font = '11px Inter, sans-serif';
    ctx.fillText(prim.label.slice(0, 32), t0.sx + 4, t0.sy + 14);
    ctx.restore();
  }
}

function renderApp() {
  const root = document.getElementById('app');
  if (!root) return;

  if (!client.currentUser) {
    root.innerHTML = renderAuthScreen();
    bindAuthEvents();
    return;
  }

  if (!client.snapshot) {
    root.innerHTML = renderSetupScreen();
    bindSetupEvents();
    return;
  }

  const snap = client.snapshot;
  const w = snap.world;
  const rw = snap.runway;

  let activeTabContent = '';
  if (client.activeTab === 'Plants') activeTabContent = renderPlantsTab(w);
  else if (client.activeTab === 'Markets') activeTabContent = renderMarketsTab(w);
  else if (client.activeTab === 'Floor') activeTabContent = renderFloorTab(w);
  else if (client.activeTab === 'Timeline') activeTabContent = renderTimelineTab(w);
  else if (client.activeTab === 'Ledger') activeTabContent = renderLedgerTab(w, snap);
  else if (client.activeTab === 'People') activeTabContent = renderPeopleTab(w);

  root.innerHTML = `
    <div class="top-bar">
      <div style="display:flex; align-items:center; gap:12px;">
        <button class="btn" id="btn-back-setup">← Games</button>
        <strong style="color:var(--accent); font-size:15px;">Factory Sim</strong>
        <span class="metric-pill">
          <span class="metric-label">Sim Clock</span>
          <span class="metric-val">${formatSimDate(w.now)}</span>
        </span>
        <button class="btn" id="btn-step-6h" title="Step 6 hours">+6h Step</button>
        <button class="btn btn-primary" id="btn-skip-event" title="Skip ahead to next pipeline/order event (Key: S)">
          Skip Ahead to Next Event ⏭
        </button>
      </div>

      <div class="top-metrics">
        <div class="metric-pill">
          <span class="metric-label">Cash</span>
          <span class="metric-val" style="color:${rw.cash_now >= 0 ? 'var(--green)' : 'var(--red)'}">
            ${formatMoney(rw.cash_now, client.currency)}
          </span>
        </div>
        <div class="metric-pill">
          <span class="metric-label">Runway (P10/P50/P90)</span>
          <span class="metric-val">${rw.runway_days_p10}d / ${rw.runway_days_p50}d / ${rw.runway_days_p90}d</span>
        </div>
        <select class="input" id="select-units" style="width:92px;">
          <option value="metric" ${client.unitSystem === 'metric' ? 'selected' : ''}>Metric (m)</option>
          <option value="imperial" ${client.unitSystem === 'imperial' ? 'selected' : ''}>Imperial (ft)</option>
        </select>
        <select class="input" id="select-currency" style="width:76px;">
          <option value="USD" ${client.currency === 'USD' ? 'selected' : ''}>USD ($)</option>
          <option value="EUR" ${client.currency === 'EUR' ? 'selected' : ''}>EUR (€)</option>
        </select>
        <button class="btn" id="btn-open-problem-modal">Report a Problem</button>
      </div>
    </div>

    <div class="tab-strip">
      <div class="tabs-left">
        ${['Plants', 'Markets', 'Floor', 'Timeline', 'Ledger', 'People']
          .map(
            (t, i) => `
          <button class="tab-btn ${client.activeTab === t ? 'active' : ''}" data-tab="${t}">
            ${i + 1}. ${t}
          </button>
        `
          )
          .join('')}
      </div>
      <div style="font-size:12px; color:var(--text-2);">
        Seed: <code>${w.game_seed}</code> ·
        Selected Plant: <strong>${client.selectedPlantId || 'None'}</strong>
      </div>
    </div>

    <div class="workspace-grid">
      <div class="main-pane">
        ${
          client.statusBanner
            ? `<div class="card" style="border-color:var(--red); color:#fca5a5;">${client.statusBanner.text}</div>`
            : ''
        }
        ${
          rw.bankrupt
            ? `<div class="card" style="border-color:var(--red); background:rgba(239,68,68,0.2);">
                <strong>GAME OVER — BANKRUPTCY:</strong> The company ran out of cash and credit headroom. Rewind to an earlier checkpoint on the right to try a different strategy!
              </div>`
            : ''
        }
        ${renderTutorialBanner(w)}
        ${activeTabContent}
      </div>

      <div class="detail-pane">
        ${renderRightDetailInspector(w, snap)}
      </div>
    </div>

    ${renderSkipModal()}
    ${renderProblemModal()}
  `;

  bindGameEvents();
  if (client.activeTab === 'Floor') {
    drawFloor3DCanvas();
  }
}

function bindAuthEvents() {
  document.getElementById('tab-auth-login')?.addEventListener('click', () => {
    authMode = 'login';
    renderApp();
  });
  document.getElementById('tab-auth-invite')?.addEventListener('click', () => {
    authMode = 'invite';
    renderApp();
  });
  document.getElementById('tab-auth-reset')?.addEventListener('click', () => {
    authMode = 'reset';
    renderApp();
  });

  document.getElementById('btn-do-login')?.addEventListener('click', async () => {
    try {
      const email = document.getElementById('login-email').value;
      const password = document.getElementById('login-password').value;
      const res = await apiFetch('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      });
      client.sessionToken = res.session_token;
      client.currentUser = res.user;
      client.statusBanner = null;
      await loadCatalogsAndGames();
    } catch (err) {
      client.statusBanner = { kind: 'error', text: err.message };
      renderApp();
    }
  });

  document.getElementById('btn-do-register')?.addEventListener('click', async () => {
    try {
      const invite_token = document.getElementById('inv-token').value;
      const email = document.getElementById('inv-email').value;
      const password = document.getElementById('inv-password').value;
      const res = await apiFetch('/api/auth/register', {
        method: 'POST',
        body: JSON.stringify({ invite_token, email, password }),
      });
      client.sessionToken = res.session_token;
      client.currentUser = res.user;
      client.statusBanner = null;
      await loadCatalogsAndGames();
    } catch (err) {
      client.statusBanner = { kind: 'error', text: err.message };
      renderApp();
    }
  });

  document.getElementById('btn-do-reset')?.addEventListener('click', async () => {
    try {
      const reset_token = document.getElementById('rst-token').value;
      const new_password = document.getElementById('rst-password').value;
      await apiFetch('/api/auth/reset-password', {
        method: 'POST',
        body: JSON.stringify({ reset_token, new_password }),
      });
      authMode = 'login';
      client.statusBanner = null;
      renderApp();
    } catch (err) {
      client.statusBanner = { kind: 'error', text: err.message };
      renderApp();
    }
  });
}

function bindSetupEvents() {
  document.getElementById('btn-logout')?.addEventListener('click', async () => {
    await apiFetch('/api/auth/logout', { method: 'POST' }).catch(() => {});
    client.currentUser = null;
    client.sessionToken = null;
    client.snapshot = null;
    renderApp();
  });

  document.getElementById('btn-create-game')?.addEventListener('click', async () => {
    const scenario_name = document.getElementById('new-scenario').value;
    const game_seed = Number(document.getElementById('new-seed').value) || 2026;
    const auto_pause_on_alerts = document.getElementById('new-autopause').checked;
    const snap = await apiFetch('/api/games', {
      method: 'POST',
      body: JSON.stringify({ scenario_name, game_seed, auto_pause_on_alerts }),
    });
    client.applyServerSnapshot(snap);
  });

  document.querySelectorAll('.btn-open-game').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const gid = btn.getAttribute('data-gid');
      const snap = await apiFetch(`/api/games/${gid}`);
      client.applyServerSnapshot(snap);
    });
  });

  document.getElementById('btn-gen-invite')?.addEventListener('click', async () => {
    try {
      const res = await apiFetch('/api/auth/invite', {
        method: 'POST',
        body: JSON.stringify({ role: 'player' }),
      });
      generatedInviteToken = res.invite_token;
      renderApp();
    } catch (err) {
      alert(err.message);
    }
  });

  document.getElementById('btn-gen-reset')?.addEventListener('click', async () => {
    try {
      const email = document.getElementById('rst-target-email').value || 'admin@factory.local';
      const res = await apiFetch('/api/auth/reset-link', {
        method: 'POST',
        body: JSON.stringify({ email }),
      });
      generatedResetToken = res.reset_token;
      renderApp();
    } catch (err) {
      alert(err.message);
    }
  });
}

function bindGameEvents() {
  document.getElementById('btn-back-setup')?.addEventListener('click', async () => {
    client.snapshot = null;
    await loadCatalogsAndGames();
  });

  document.querySelectorAll('.tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => client.setTab(btn.getAttribute('data-tab')));
  });

  document.getElementById('btn-step-6h')?.addEventListener('click', () => triggerStepHours(6));
  document.getElementById('btn-skip-event')?.addEventListener('click', () => triggerSkipAhead(7));

  document.getElementById('select-units')?.addEventListener('change', (e) => {
    client.unitSystem = e.target.value;
    client.notify();
  });
  document.getElementById('select-currency')?.addEventListener('change', (e) => {
    client.currency = e.target.value;
    client.notify();
  });

  // Tutorial helper button
  document.getElementById('btn-tutorial-auto-step')?.addEventListener('click', async () => {
    const step = client.snapshot.world.tutorial_step;
    if (step === 1) {
      await sendCommand({
        LeasePlant: {
          plant_id: 'plant-1',
          listing_id: 'bldg-suburban-bay',
          name: 'Fremont Plant #1',
        },
      });
      client.setTab('Floor');
    } else if (step === 2) {
      await sendCommand({
        PlaceCell: {
          plant_id: client.selectedPlantId || 'plant-1',
          cell_id: 'cnc-1',
          team: 'Machining',
          catalog_id: 'cnc-3axis-vmc',
          addons: ['probing'],
          x_mm: 5000,
          y_mm: 4000,
          rotation_deg: 0,
        },
      });
      await sendCommand({
        PlaceCell: {
          plant_id: client.selectedPlantId || 'plant-1',
          cell_id: 'pack-1',
          team: 'FinishingPacking',
          catalog_id: 'pack-manual-station',
          addons: [],
          x_mm: 10000,
          y_mm: 4000,
          rotation_deg: 0,
        },
      });
      client.setTab('People');
    } else if (step === 3) {
      await sendCommand({
        HireWorker: {
          plant_id: client.selectedPlantId || 'plant-1',
          worker_id: 'worker-1',
          role: 'CNC & Logistics Operator',
          wage_per_hour_tenth_cents: 32000,
        },
      });
      client.setTab('Markets');
    } else if (step === 4) {
      await sendCommand({
        ConfigureMarketPolicy: {
          plant_id: client.selectedPlantId || 'plant-1',
          market_id: 'mkt-prototyping-online',
          enabled: true,
          min_payout_per_hour_tenth_cents: 80000,
          price_multiplier_pct: 100,
          promised_lead_days: 7,
        },
      });
    } else if (step === 5) {
      await triggerSkipAhead(18);
    }
  });

  document.getElementById('btn-tutorial-skip')?.addEventListener('click', () => {
    client.snapshot.world.tutorial_step = 0;
    client.notify();
  });

  // Plants tab events
  document.querySelectorAll('.btn-select-plant').forEach((btn) => {
    btn.addEventListener('click', () => client.selectPlant(btn.getAttribute('data-pid')));
  });
  document.querySelectorAll('.btn-lease-building').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const lid = btn.getAttribute('data-lid');
      const bname = btn.getAttribute('data-bname');
      const nextIdx = Object.keys(client.snapshot.world.plants || {}).length + 1;
      const pid = `plant-${nextIdx}`;
      await sendCommand({
        LeasePlant: { plant_id: pid, listing_id: lid, name: `${bname} (#${nextIdx})` },
      });
      client.selectPlant(pid);
    });
  });

  // Markets tab events
  document.querySelectorAll('.btn-toggle-market').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const mid = btn.getAttribute('data-mid');
      const enabled = btn.getAttribute('data-enable') === 'true';
      const priceEl = document.querySelector(`.mkt-price-${mid}`);
      const leadEl = document.querySelector(`.mkt-lead-${mid}`);
      await sendCommand({
        ConfigureMarketPolicy: {
          plant_id: client.selectedPlantId,
          market_id: mid,
          enabled,
          min_payout_per_hour_tenth_cents: 80000,
          price_multiplier_pct: Number(priceEl?.value) || 100,
          promised_lead_days: Number(leadEl?.value) || 7,
        },
      });
    });
  });

  document.querySelectorAll('.btn-unlock-cert').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const cid = btn.getAttribute('data-cid');
      await sendCommand({
        UnlockMarketCert: { plant_id: client.selectedPlantId, cert_id: cid },
      });
    });
  });

  // Floor tab events
  document.querySelectorAll('.btn-team-filter').forEach((btn) => {
    btn.addEventListener('click', () => {
      activeTeamFilter = btn.getAttribute('data-team');
      renderApp();
    });
  });

  document.getElementById('btn-toggle-planning')?.addEventListener('click', () => {
    client.togglePlanningMode();
  });
  document.getElementById('btn-plan-undo')?.addEventListener('click', () => {
    client.undoPlannedChange();
  });
  document.getElementById('btn-plan-clear')?.addEventListener('click', () => {
    client.clearPlannedCells();
  });
  document.getElementById('btn-plan-execute')?.addEventListener('click', async () => {
    const drafts = [...client.plannedCells];
    client.clearPlannedCells();
    for (const d of drafts) {
      const nextNum = Object.keys(client.snapshot.world.cells || {}).length + 1;
      await sendCommand({
        PlaceCell: {
          plant_id: d.plant_id,
          cell_id: `cell-${nextNum}`,
          team: d.team,
          catalog_id: d.catalog_id,
          addons: d.addons || [],
          x_mm: d.x_mm,
          y_mm: d.y_mm,
          rotation_deg: d.rotation_deg,
        },
      });
    }
  });

  document.querySelectorAll('.btn-place-catalog-cell').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const catId = btn.getAttribute('data-cat');
      const cat = client.catalogs.cells.find((c) => c.catalog_id === catId);
      if (!cat || !client.selectedPlantId) return;

      const existingCount =
        Object.values(client.snapshot.world.cells || {}).filter(
          (c) => c.plant_id === client.selectedPlantId
        ).length + client.plannedCells.length;
      const col = existingCount % 4;
      const row = Math.floor(existingCount / 4);
      const x_mm = 4200 + col * 3800;
      const y_mm = 1500 + row * 3800;

      if (client.planningMode) {
        client.addPlannedCellDraft(cat, x_mm, y_mm, 0, true, []);
      } else {
        const cellId = `${cat.team.toLowerCase().slice(0, 4)}-${existingCount + 1}`;
        await sendCommand({
          PlaceCell: {
            plant_id: client.selectedPlantId,
            cell_id: cellId,
            team: cat.team,
            catalog_id: cat.catalog_id,
            addons: [],
            x_mm,
            y_mm,
            rotation_deg: 0,
          },
        });
      }
    });
  });

  document.getElementById('btn-order-stock')?.addEventListener('click', async () => {
    await sendCommand({
      OrderRawStock: {
        plant_id: client.selectedPlantId,
        sku: 'blank-alu',
        quantity: 6,
      },
    });
    await sendCommand({
      OrderRawStock: {
        plant_id: client.selectedPlantId,
        sku: 'carton-box',
        quantity: 6,
      },
    });
  });

  // People tab events
  document.getElementById('btn-hire-operator')?.addEventListener('click', async () => {
    const nextIdx = Object.keys(client.snapshot.world.workers || {}).length + 1;
    await sendCommand({
      HireWorker: {
        plant_id: client.selectedPlantId,
        worker_id: `worker-${nextIdx}`,
        role: 'Multi-Cell Operator & Logistics',
        wage_per_hour_tenth_cents: 32000,
      },
    });
  });

  // Ledger tab events
  document.getElementById('btn-take-loan')?.addEventListener('click', async () => {
    const financeableCell = Object.values(client.snapshot.world.cells || {}).find(
      (c) => c.team === 'Machining' || c.team === 'Molding'
    );
    if (!financeableCell) {
      client.statusBanner = {
        kind: 'error',
        text: 'Place a financeable CNC or Molding machine on the Floor first.',
      };
      renderApp();
      return;
    }
    const loanNum = Object.keys(client.snapshot.world.loans || {}).length + 1;
    await sendCommand({
      TakeEquipmentLoan: {
        loan_id: `loan-${loanNum}`,
        principal_tenth_cents: 45_000_000,
        collateral_cell_id: financeableCell.cell_id,
      },
    });
  });

  // Right detail inspector & checkpoints
  document.querySelectorAll('[data-select-cell]').forEach((el) => {
    el.addEventListener('click', () => {
      client.selectEntity(el.getAttribute('data-select-cell'));
    });
  });

  document.getElementById('btn-save-checkpoint')?.addEventListener('click', async () => {
    const name = document.getElementById('ck-name-input').value || 'Manual Checkpoint';
    const gid = client.snapshot.world.game_id;
    const snap = await apiFetch(`/api/games/${gid}/checkpoint`, {
      method: 'POST',
      body: JSON.stringify({ name, is_autosave: false }),
    });
    client.applyServerSnapshot(snap);
  });

  document.querySelectorAll('.btn-rewind-ck').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const checkpoint_id = btn.getAttribute('data-ckid');
      const gid = client.snapshot.world.game_id;
      const snap = await apiFetch(`/api/games/${gid}/rewind`, {
        method: 'POST',
        body: JSON.stringify({ checkpoint_id }),
      });
      client.applyServerSnapshot(snap);
    });
  });

  // Modals
  document.getElementById('btn-close-skip-modal')?.addEventListener('click', () => {
    showSkipModal = false;
    renderApp();
  });

  document.getElementById('btn-open-problem-modal')?.addEventListener('click', () => {
    showProblemModal = true;
    renderApp();
  });

  document.getElementById('btn-close-problem-modal')?.addEventListener('click', () => {
    showProblemModal = false;
    renderApp();
  });

  document.getElementById('btn-submit-problem')?.addEventListener('click', async () => {
    const note = document.getElementById('problem-note-input').value || 'User bug report';
    const gid = client.snapshot.world.game_id;
    lastProblemBundle = await apiFetch(`/api/games/${gid}/report`, {
      method: 'POST',
      body: JSON.stringify({ note }),
    });
    renderApp();
  });
}

bootstrapSession();
