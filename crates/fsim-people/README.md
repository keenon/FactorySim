# `fsim-people`

People Ops vertical slice: worker roster, domain ramp-up curve (`1 - e^(-t/tau)`), shift task priorities, and the central iPad task dispatch algorithm.

- **State & Caching**: Stateless (`WorkerState` and `PendingFloorTask` are caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-geom`, `fsim-layout-graph`, `fsim-motion`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `WorkerState`, `PendingFloorTask` queue, and `TransportGraph` waypoints -> Produces `ActiveWorkerTask` with timed `MotionSegment`s and iPad dispatch events

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns plant workers, their virtual iPad devices (`plant/<p>/person/<w>/ipad`), task priority selection, and ramp-up productivity scaling.
- **Core Invariants**:
  - **Non-Interruptible Tasks**: Once a task is dispatched to a worker's iPad (`worker.active_task.is_some()`), the worker cannot be interrupted mid-task.
  - **Stochastic Communication & Task Times**: Both iPad reading/acknowledgement (`ipad_ack_done_at`) and physical task execution (`work_done_at`) sample once from `StreamRng` and store their completion timestamps.
- **Conditioning Variables**: Cumulative domain hours (`worked_ms_in_domain`), shift priority order, and walking distance along the transport graph.

## Algorithms Implemented (Do Not Duplicate)
- **Domain Ramp-Up Curve** (`reference::reference_ramp_up_permille`): Evaluates $1 - 0.75 e^{-t / 80\text{h}}$ productivity scaling in permille (`250..=1000`).
- **Central iPad Task Dispatcher** (`reference::reference_select_next_task_index`, `dispatch_task_to_worker`): Selects the highest-priority unblocked floor task (breaking ties by age and walking distance) and generates timed walking segments plus stochastic task duration.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `PendingFloorTask`, `ActiveWorkerTask`, `WorkerState`, `default_task_priorities()`, `dispatch_task_to_worker()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference ramp-up and priority dispatch oracle (`reference_ramp_up_permille`, `reference_select_next_task_index`).
