import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { createHttpApi } from './api';
import { FloorScene } from './scene/FloorScene';
import { GameClient } from './state/GameClient';
import { GameClientContext } from './state/useGameClient';
import { App } from './ui/App';
import { FloorSceneContext } from './ui/floorSceneContext';

const client = new GameClient(createHttpApi());
const floorScene = new FloorScene(client);

createRoot(document.getElementById('app')!).render(
  <StrictMode>
    <GameClientContext.Provider value={client}>
      <FloorSceneContext.Provider value={floorScene}>
        <App />
      </FloorSceneContext.Provider>
    </GameClientContext.Provider>
  </StrictMode>,
);

void client.boot();
