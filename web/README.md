# `web` — Factory Sim viewer & command surface

React + TypeScript single-page app. Renders server state and sends commands; it never simulates.

- **State & Caching**: one central `GameClient` (server mirror + UI-local state); persistent `FloorScene` (Three.js)
- **Dependencies**: `react`, `react-dom`, `three` | Dev: `typescript`, `esbuild`, `@types/*`
- **Data Flow**: `fsim-server` JSON (`ClientGameSnapshot`, `StaticCatalogBundle`) -> `GameClient` -> React views + `FloorScene`; user actions -> `GameClient` methods -> `PlantCommand`s

## Design Intent & Assumptions
- **Encapsulation Boundary**: Rust owns simulation, validation and what cells look like (`CellModel` from team crates). The browser owns presentation, camera, selection and planning-mode drafts (controller-side, not sent until commit).
- **Core Invariants**:
  - The server mirror (`snapshot`, `catalogs`) changes only via `GameClient.applySnapshot` / lobby loads.
  - React re-renders on `GameClient.version` (`useSyncExternalStore`). The 3D view re-syncs on `sceneVersion`; live drags bump only `sceneVersion`, so dragging never re-renders React.
  - Local placement checks (`logic/placement.ts`) are an instant preview; the server's `/validate-placement` verdict replaces them on drop.
- **Execution Assumptions**: Scene is Z-up in meters (sim mm × 0.001). Model geometries/materials are shared and cached; entities rebuild only when their model key changes.

## Module Map
- **`src/types.ts`**: hand-mirrored wire types of the Rust serde shapes (keep in sync with `fsim-core`, `fsim-protocols`, `fsim-plant-host`, `fsim-server`).
- **`src/api.ts`**: `Api` interface + `createHttpApi()`.
- **`src/state/GameClient.ts`**: central state class; **`useGameClient.ts`**: React subscription hook.
- **`src/logic/`** (pure, tested): `format.ts`, `motion.ts` (`poseAt`, mirrors `fsim-motion`), `placement.ts` (footprints, preview checks, free-spot search).
- **`src/scene/`**: `coords.ts` (pure mm↔m, cell transforms), `modelBuilder.ts` (`CellModel` → `THREE.Group`, fallback boxes, footprint outlines), `FloorScene.ts` (renderer, perspective camera + OrbitControls/WASD, lights/shadows, building, picking, drag-to-place, labels, spindle animation).
- **`src/ui/`**: `App.tsx` (shell + shortcuts), `AuthScreen`, `SetupScreen`, `TopBar`, `TutorialBanner`, `Inspector`, `Modals`, `Chips`, `tabs/{Plants,Markets,Floor,Timeline,Ledger,People}Tab.tsx`.
- **`dist/`**: `index.html`, `styles.css` (light theme); `app.js` is the build output (gitignored).

## Commands
`npm run build` (bundle to `dist/app.js`), `npm run watch`, `npm run typecheck`, `npm test` (esbuild-bundled `node --test` over `tests/*.test.ts`: logic, coords, model builder, `GameClient` planning flow with a fake `Api`).
