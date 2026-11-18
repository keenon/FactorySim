# `fsim-sales`

Markets and demand vertical slice: Online Prototyping Marketplace (Xometry-style acceptance policy), unlockable regulated markets (Aerospace AS9100, Medical ISO 13485, Commercial Contract Mfg), certifications, and late/out-of-tolerance settlement penalties.

- **State & Caching**: Stateless (`MarketBiddingPolicy` and `CustomerOrder` are caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `MarketBiddingPolicy`, reputation permille, and `StreamRng` -> Produces `CustomerOrder` arrivals and shipment payout/reputation settlements

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns market definitions, site vs. company-wide certifications (`cert-iso9001`, `cert-as9100`, `cert-iso13485`, `cert-itar`), win probability, and shipment settlement.
- **Core Invariants**:
  - **Monotonic Price/Reputation Response**: Win probability decreases monotonically as `price_multiplier_pct` increases and increases monotonically with reputation permille and shorter `promised_lead_days`.
  - **Late & Out-of-Tolerance Penalties**: Out-of-spec shipments incur a chargeback (`-25%` of reference payout) and `-40` reputation; late shipments incur a `30%` payout reduction and `-15` reputation.
- **Conditioning Variables**: `price_multiplier_pct`, `promised_lead_days`, `reputation_permille`, and `min_payout_per_hour`.

## Algorithms Implemented (Do Not Duplicate)
- **Bid Win Probability & Shipment Settlement** (`reference::reference_win_probability_permille`, `reference::reference_settle_shipment`): Computes order win odds and shipment payout/reputation deltas.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `MarketDefinition`, `MarketBiddingPolicy`, `CustomerOrder`, `CertificationSpec`, `starter_certifications()`, `starter_markets()`, `sample_customer_order()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference win probability and settlement oracle (`reference_win_probability_permille`, `reference_settle_shipment`).
