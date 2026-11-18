import { createContext, useContext } from 'react';
import type { FloorScene } from '../scene/FloorScene';

export const FloorSceneContext = createContext<FloorScene | null>(null);

export function useFloorScene(): FloorScene {
  const scene = useContext(FloorSceneContext);
  if (!scene) throw new Error('FloorSceneContext missing');
  return scene;
}
