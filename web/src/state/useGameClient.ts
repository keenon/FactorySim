import { createContext, useContext, useSyncExternalStore } from 'react';
import type { GameClient } from './GameClient';

export const GameClientContext = createContext<GameClient | null>(null);

/** Subscribes the calling component to `GameClient.version` and returns the client. */
export function useGameClient(): GameClient {
  const client = useContext(GameClientContext);
  if (!client) throw new Error('GameClientContext missing');
  useSyncExternalStore(client.subscribe, client.getVersion);
  return client;
}
