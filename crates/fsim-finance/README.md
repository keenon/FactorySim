# `fsim-finance`

Finance vertical slice: equipment loans against resellable machines, credit lines, runway fan-chart projections (10th, 50th, 90th percentiles), and bankruptcy detection.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes cash balance, credit headroom, monthly burn, and collateral cell value -> Produces `EquipmentLoan` terms and `RunwayProjection`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns equipment loan underwriting (`financeable` cells only), monthly amortization schedules, and runway percentile projections. Double-entry bookkeeping lives in `fsim-core::DoubleEntryLedger`.
- **Core Invariants**:
  - **Bankruptcy Condition**: A game ends if and only if `cash_now + credit_available < 0` (`RunwayProjection::bankrupt == true`); otherwise play continues indefinitely.
  - **Ordered Runway Percentiles**: `runway_days_p10 <= runway_days_p50 <= runway_days_p90` for all inputs.
- **Conditioning Variables**: Collateral machine resale value, APR basis points (`apr_bps`), and rolling monthly revenue/burn variance.

## Algorithms Implemented (Do Not Duplicate)
- **Equipment Loan Amortization & Runway Percentile Estimator** (`reference::reference_monthly_loan_payment`, `reference::reference_runway_days`, `RunwayProjection::compute`): Computes monthly debt service and 10th/50th/90th percentile runway days.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `EquipmentLoan`, `RunwayProjection`.
- **[`src/reference.rs`](src/reference.rs)**: Reference loan payment and runway oracle (`reference_monthly_loan_payment`, `reference_runway_days`).
