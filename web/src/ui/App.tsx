import { useEffect } from 'react';
import { TABS } from '../state/GameClient';
import { useGameClient } from '../state/useGameClient';
import { AuthScreen } from './AuthScreen';
import { Inspector } from './Inspector';
import { ProblemModal, SkipSummaryModal } from './Modals';
import { SetupScreen } from './SetupScreen';
import { FloorTab } from './tabs/FloorTab';
import { LedgerTab } from './tabs/LedgerTab';
import { MarketsTab } from './tabs/MarketsTab';
import { PeopleTab } from './tabs/PeopleTab';
import { PlantsTab } from './tabs/PlantsTab';
import { TimelineTab } from './tabs/TimelineTab';
import { TopBar } from './TopBar';
import { TutorialBanner } from './TutorialBanner';

export function App() {
  const client = useGameClient();
  useGlobalShortcuts();

  if (client.booting) return <div className="splash">Loading Factory Sim…</div>;
  if (!client.user) return <AuthScreen />;
  if (!client.snapshot) return <SetupScreen />;

  const { runway } = client.snapshot;
  return (
    <div className="game-shell">
      <TopBar />
      <nav className="tab-strip">
        <div className="tabs">
          {TABS.map((t, i) => (
            <button key={t} className={`tab ${client.activeTab === t ? 'active' : ''}`} onClick={() => client.setTab(t)}>
              <span className="tab-key">{i + 1}</span>
              {t}
            </button>
          ))}
        </div>
        <div className="muted small">
          Seed <code>{client.snapshot.world.game_seed}</code> · Plant <strong>{client.selectedPlantId ?? 'none'}</strong>
        </div>
      </nav>
      <div className={`workspace ${client.activeTab === 'Floor' ? 'workspace-floor' : ''}`}>
        <main className="main-pane">
          {client.banner && (
            <div className="notice notice-error" onClick={() => client.dismissBanner()}>
              {client.banner} <span className="muted">(click to dismiss)</span>
            </div>
          )}
          {runway.bankrupt && (
            <div className="notice notice-error">
              <strong>Bankrupt.</strong> The company ran out of cash and credit. Rewind to a checkpoint to try another strategy.
            </div>
          )}
          <TutorialBanner />
          <ActiveTab />
        </main>
        <aside className="detail-pane">
          <Inspector />
        </aside>
      </div>
      <SkipSummaryModal />
      <ProblemModal />
    </div>
  );
}

function ActiveTab() {
  const client = useGameClient();
  switch (client.activeTab) {
    case 'Plants':
      return <PlantsTab />;
    case 'Markets':
      return <MarketsTab />;
    case 'Floor':
      return <FloorTab />;
    case 'Timeline':
      return <TimelineTab />;
    case 'Ledger':
      return <LedgerTab />;
    case 'People':
      return <PeopleTab />;
  }
}

function useGlobalShortcuts() {
  const client = useGameClient();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA' || !client.snapshot) return;
      const sel = client.selectedEntityId;
      if (e.key >= '1' && e.key <= '6') client.setTab(TABS[Number(e.key) - 1]);
      else if (e.key === ' ' && !e.repeat) {
        e.preventDefault();
        client.togglePlay();
      } else if ((e.key === 'n' || e.key === 'N') && !e.repeat) void client.skipAhead(7); else if ((e.key === 'r' || e.key === 'R') && sel?.startsWith('plan-')) client.rotateDraft(sel);
      else if ((e.key === 'Delete' || e.key === 'Backspace') && sel?.startsWith('plan-')) client.removeDraft(sel);
      else if ((e.metaKey || e.ctrlKey) && e.key === 'z') {
        e.preventDefault();
        client.undo();
      } else if (e.key === 'Escape') client.select(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [client]);
}
