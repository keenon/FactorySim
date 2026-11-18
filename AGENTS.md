# AGENTS.md

Conventions for all code in the factory sim workspace: the Rust simulation core and solvers, the Rust game/MCTS and twin servers, and the TypeScript web app.

Crate names below use the `fsim-` prefix (placeholder; rename to taste).

## For all our code

- We prefer many smaller crates over fewer larger ones. A team owns several small crates, not one big one.
- We prefer stateless crates as much as possible. The simulation core is a pure step function, `step(state, inputs) -> (new state, events)`; state is caller-owned.
- We want simple, small-surface-area APIs.
- We use TDD, and fuzz test where appropriate. Fuzz tests are hidden behind `--features fuzz-tests` so they don't slow down the main suite; they exist to find bugs.
- We keep state isolated in as few places as possible. Mutable state lives only in game actors (game server) and the event store and snapshots (twin service). Everything else takes state as input and returns new state or events.
- We keep algorithms correct in all cases, not by accumulating edge cases or tweaking thresholds. This is especially dangerous in geometry code (layout, clearance checks, collision, path planning for people and forklifts), where it is easy to keep adding special cases and heuristic thresholds until the code can't be reasoned about and will never be fully correct. We always choose a slower algorithm if it is guaranteed correct in all cases and the faster one is not. We choose simplicity over a string of edge cases wherever possible.
- When optimizing (better algorithms, multi-threaded CPU, GPU), we always first have a single-threaded, simple, well-tested, unoptimized CPU reference that we can fuzz test the optimized code against. We optimize the Brendan Gregg way, through measurement: run the benchmarks and profilers to confirm the added complexity actually pays. Iterate: profile, find the resource bottleneck (often cache misses rather than CPU time), attack it (memory layout, algorithms, parallelism), profile again.
- We follow Ousterhout: "classes should be martyrs for complexity." Encapsulate complexity and expose clean, simple interfaces at every layer of the stack.
- Git history is a useful tool, so use it. Commit with clear messages.

## Simulation-specific rules

- **Determinism is a correctness property.** Same seed + same inputs = bit-identical output. Use ordered maps only (no `HashMap` iteration order in sim paths), one seeded random stream per module (keyed by module and entity), no wall-clock time, and careful floating-point math. A golden-replay test in CI must fail if determinism breaks.
- **One definition of every shared type.** Protocol messages (work orders, purchase orders, maintenance tickets, NCRs, shipments) are defined once, in `fsim-protocols`, and versioned. World-state types are defined once in the core crates. Never redefine a near-copy of an existing type.
- **Fidelity levels share one contract.** Each module trait has a conformance test suite that every fidelity level must pass. A new fidelity level is a new implementation of the trait, not a fork of the trait.
- **Reference oracles vs. calibration tests are different things.**
  - A *reference oracle* (`src/reference.rs`) is a simple, unoptimized implementation of the *same semantics* as the optimized code. Optimized code must match it exactly (or within a stated numerical tolerance) under differential fuzz tests.
  - A *lower fidelity level* or the *coarse sim* is an approximation, not an oracle. It is checked by statistical calibration tests against a higher fidelity level (distributional agreement within stated bounds), never by exact-match fuzzing.
- **Conditional models declare their conditioning variables.** Every model that predicts outcomes states exactly which conditions it conditions on; this is part of its public contract and is documented in its README.
- **Games never write real state.** Only the twin service writes to the real-factory record, and only through the ingestion and ops paths.

## Documentation for agents

We don't want to repeat ourselves (within reason: DRY taken too far adds complexity). We don't want parallel implementations of the same algorithm that inevitably drift into subtly different behavior, or many redefinitions of the same data structure with slight deviations.

The point of many small stateless crates with simple APIs is a standardized per-crate `README.md` that agents can ingest without using much context, so they can get a detailed view of the whole codebase without reading individual files.

- When you make changes, update the crate's `README.md`.
- When you implement something new, read all existing `README.md` files first.

### Crate `README.md` guidelines

Keep them reasonably concise, about 45–200 lines per crate. This applies to server crates too (`fsim-server`, `fsim-twin`).

1. **Strictly local ground truth (no central or cross-crate drift).**
   - Document only facts the crate owns: its upstream dependencies (from its own `Cargo.toml`), internal representations, algorithms, and input/output data-flow contracts.
   - No central architecture README, and no lists of downstream consumers in upstream READMEs. They go stale and cause merge conflicts when agents work in parallel. Agents read all crate READMEs together and reconstruct the dependency graph from each crate's local upstream declarations.
2. **Compact metadata header (no tutorial code blocks or CLI boilerplate).**
   - No multi-line Quickstart blocks and no repetitive `cargo test` / `cargo bench` boilerplate.
   - Start with a one-sentence responsibility summary, then:
     - **State & Caching**: e.g. `100% Stateless`, `Stateless + caller-owned FooCache`, or `Stateful game actor FSM`.
     - **Dependencies**: direct workspace `fsim-*` dependencies from `Cargo.toml`, plus notable external crates (`tokio`, `axum`, `im`, `rand_chacha`, etc.).
     - **Data Flow**: upstream types consumed -> types produced.
3. **Design intent, principles and assumptions (no speculative future predictions).**
   - No "where to generalize" section. State the crate's design intent, principles and local assumptions so agents can decide case by case whether to generalize an existing API or put logic elsewhere:
     - **Encapsulation Boundary**: what complexity this crate hides, and what explicitly belongs outside it.
     - **Core Invariants**: mathematical, geometric, physical, accounting (e.g. ledger always balances), or state-machine invariants guaranteed across all inputs.
     - **Execution & Data-Flow Assumptions**: data layout, locality, cost/latency tier (e.g. µs coarse-sim rollout path vs. ms detailed-sim step vs. s CAM-level estimate), batch vs. incremental, caller-owned cache keying, random-stream usage.
     - For model crates: **Conditioning Variables** and the fidelity level(s) implemented.
4. **Algorithms implemented ("Do Not Duplicate").**
   - A concise list of the complex mathematical, geometric, physical, statistical, optimization or search algorithms in the crate, naming entry points and correctness guarantees, including reusable helpers so other crates don't re-implement them.
5. **Compact, file-linked public API and reference oracle.**
   - Group canonical types, traits and signatures (`fn(Inputs) -> Outputs`) by `src/module.rs`, distinguishing types defined here from upstream types consumed or re-exported.
   - State what `src/reference.rs` implements as the simple single-threaded oracle, and what invariants `--features fuzz-tests` checks against it. For model crates, also state which calibration tests compare it to which higher fidelity level.

### Crate `README.md` template

```markdown
# `crate-name`

One-sentence summary of the crate's responsibility.

- **State & Caching**: Stateless (or: Stateless with caller-owned `FooCache` / Stateful FSM)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `im`, `rand_chacha`
- **Data Flow**: Consumes `InputTypes` -> Produces `OutputTypes`

## Design Intent & Assumptions
- **Encapsulation Boundary**: What complexity this crate hides and what does *not* belong here.
- **Core Invariants**: Mathematical/physical/accounting/structural invariants guaranteed across all inputs.
- **Execution & Granularity Assumptions**: Data layout, locality, cost tier (µs vs. ms), caching and random-stream assumptions.
- **Conditioning Variables** (model crates only): What the model conditions on; fidelity level(s) implemented.

## Algorithms Implemented (Do Not Duplicate)
- **[Algorithm Name]** (`module::item`): Concise description of the algorithm, representation, and exactness guarantee.

## Public API & Reference Oracle
- **[`src/module.rs`](src/module.rs)**: `KeyType`, `key_fn(a: &A) -> B`.
- **[`src/reference.rs`](src/reference.rs)**: Simple unoptimized oracle for X; differential fuzz invariants tested under `--features fuzz-tests`. Calibration tests against: (higher fidelity level, if any).
```

## Frontend

React and TypeScript are great. The challenge with React is managing state in a way React knows how to update. `useState()` hooks are only good for small cosmetic state; anything more complex becomes impossible to debug, and invisible mount/unmount cycles break things. Pure Redux makes data structures awkward to work with, and frameworks like MobX introduce weird bugs, although reintroducing OOP is generally good.

Our approach:

- OOP methods on our state, rooted in one central state class. Methods modify the state and trigger a React rerender.
- As much logic as possible is factored into pure functions called by the OOP core.
- `useState()` in components only for minimal cosmetic state.
- The OOP core and the pure helpers are unit tested extensively. No "change detector" tests for the UI layer, but anything complex and semantic is worth testing if the test doesn't just check the fidelity of our mock (e.g. packing many entities into canvas or WebGL buffers is worth breaking out as a function and testing).

Specific to this project:

- **The server owns simulation state.** The browser never simulates. The central client state class holds (a) a mirror of server state, updated only by applying server diffs, and (b) UI-local state such as selection, camera and panel layout. User actions call methods on the state class that send commands to the server; they never mutate the mirrored sim state directly.
- **Complex logic lives in Rust.** TypeScript is for the minimal stateful core plus pure functions with an excellent reason not to be Rust (e.g. they touch DOM or canvas APIs). Motion interpolation between server diffs is a pure, tested function.
- **The Rust workspace can run everything headless**: games, MCTS, solvers, ingestion and scoring. The web app is only a viewer and command surface on top.

## Iterative experimentation and logs

Several kinds of work need iterative experimentation, measurement and action: systems performance optimization, ML, and model calibration. Each crate keeps a log per kind of loop, inside the crate:

- `OPTIMIZATION_LOG.txt`: performance work.
- `ML_LOG.txt`: learned components (e.g. value estimates for MCTS).
- `CALIBRATION_LOG.txt`: fitting and validating conditional models against data (benchmarks before a factory exists, real history after).

Cross-boundary problems (WebSocket/serialization, server-to-browser latency, ingestion throughput) are triaged in the server crates' logs before being isolated to a specific downstream crate.

When running an iterative loop, follow this cycle strictly:

1. **Always identify the bottleneck first (what, where, and why).**
   - Never guess or optimize blindly. Before writing optimization or experiment code, measure to find the active limiting bottleneck, the exact code or pipeline stage hitting it, and why:
     - **Systems performance**: the hardware/system bottleneck (CPU compute or branch mispredicts, memory bandwidth, cache misses and pointer chasing, allocator churn, syscalls or lock contention, disk I/O, network or serialization overhead, main-thread UI blocking), pinpointed by profiling, hardware counters or phase timers.
     - **ML**: the learning/system bottleneck (data quantity, diversity or label noise; model capacity; training throughput or dataloader starvation; optimization dynamics), pinpointed by loss slicing, ablations or throughput profiling.
     - **Calibration**: where the model's conditional predictions are worst (which conditions, which outcomes), and why: missing conditioning variable, wrong distribution family, censoring not handled, too little data in that regime.
   - We don't always have to fix the bottleneck. Proving where it is and why it is an irreducible limit, an external constraint, or not worth the added complexity is a valid, successful conclusion.
2. **Isolate a fast, deterministic headless reproducer.**
   - Before iterating on fixes (even for UI symptoms like a frozen viewer), isolate the bottleneck into a repeatable headless `--release` benchmark, CLI harness, or evaluation test, so every iteration is measured under identical conditions in seconds.
3. **One hypothesis at a time; correctness before speed.**
   - State one explicit hypothesis aimed at the identified bottleneck: "Because [code X] hits [bottleneck Y] due to [reason Z], changing [mechanism] will reduce [metric]."
   - For systems optimizations, unit tests, the golden-replay determinism test, and differential fuzz tests (`--features fuzz-tests` against `src/reference.rs`) must pass before any benchmark speedup is trusted.
4. **Measure, decide, and `git revert` unkept experiments (zero slop accumulation).**
   - Re-run the benchmark or evaluation to measure the top-line metric and whether the bottleneck was removed or moved elsewhere.
   - **If KEPT**: commit the clean implementation (updating the crate's `README.md` if data flow or assumptions changed) together with the new log entry.
   - **If REJECTED or INCONCLUSIVE**: if the experiment didn't move the needle, or added complexity the measured gain doesn't justify, `git revert` / reset all experimental code immediately, so we never accumulate unused, incomplete or dead experiment code. Then commit only the log update, so future runs don't repeat the dead end.
   - **If STOPPED**: when the target is reached, or the remaining bottleneck can't reasonably be fixed without violating simplicity, stop and log the final bottleneck state.
5. **What to record in each log entry.**
   - **Git hash and command**: baseline commit hash and the exact benchmark, profiling or evaluation command.
   - **Identified bottleneck**: the limiting resource or regime, the exact code or data path responsible, and why it occurs.
   - **Hypothesis**: the specific change tested and expected impact.
   - **Results**: before vs. after metrics (wall-clock time, throughput, cache/memory/IPC metrics, validation loss, or scoring-rule values such as log score or CRPS).
   - **Action taken**: `KEPT (<new-commit-hash>)`, `REVERTED (code reverted to <baseline-hash>, logged only)`, or `STOPPED (bottleneck identified; no further action)`.