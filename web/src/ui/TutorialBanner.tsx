import type { GameClient } from '../state/GameClient';
import { useGameClient } from '../state/useGameClient';

const STEPS: Record<number, string> = {
  1: 'Lease your first plant on the Plants tab.',
  2: 'On the Floor tab, add a 3-Axis VMC and a Manual Pack Station, drag them into place, then Commit plan.',
  3: 'Hire one operator on the People tab — they get an iPad for task dispatch.',
  4: 'Turn on the Online Prototyping Marketplace on the Markets tab.',
  5: 'Press "Skip to next event" and watch orders chain backward and revenue land.',
};

async function doStep(client: GameClient): Promise<void> {
  const plant = client.selectedPlantId ?? 'plant-1';
  switch (client.world?.tutorial_step) {
    case 1:
      await client.send({ LeasePlant: { plant_id: 'plant-1', listing_id: 'bldg-suburban-bay', name: 'Fremont Plant #1' } });
      client.setTab('Floor');
      break;
    case 2:
      client.setTab('Floor');
      client.addDraft('cnc-3axis-vmc');
      client.addDraft('pack-manual-station');
      await client.commitDrafts();
      client.setTab('People');
      break;
    case 3:
      await client.send({ HireWorker: { plant_id: plant, worker_id: 'worker-1', role: 'CNC & Logistics Operator', wage_per_hour_tenth_cents: 32_000 } });
      client.setTab('Markets');
      break;
    case 4:
      await client.send({
        ConfigureMarketPolicy: {
          plant_id: plant,
          market_id: 'mkt-prototyping-online',
          enabled: true,
          min_payout_per_hour_tenth_cents: 80_000,
          price_multiplier_pct: 100,
          promised_lead_days: 7,
        },
      });
      break;
    case 5:
      client.setTab('Floor');
      await client.skipAhead(18);
      break;
  }
}

export function TutorialBanner() {
  const client = useGameClient();
  const step = client.world?.tutorial_step ?? 0;
  if (!step) return null;
  return (
    <div className="tutorial">
      <div>
        <span className="chip chip-accent">Tutorial {step}/5</span> {STEPS[step]}
      </div>
      <button className="btn btn-primary btn-sm" disabled={client.busy} onClick={() => void doStep(client)}>
        Do this step for me
      </button>
    </div>
  );
}
