Factory Sim: Project Overview
Nov 16, 2026 · @Keenon
Motivation
We are building a realistic factory simulator, played as a top-down 2D game, so we can learn how to run a manufacturing business before we build or instrument a real one.
• Commodity markets are transparent. Prices and product types for Amazon goods, job shop work and contract manufacturing are largely set by the market. A manufacturer controls only its own costs, volumes and timing, which makes those worth understanding in detail.
• Predictable problems are worth finding in simulation. Lead times, bottlenecks, cash crunches, permits and staffing problems are predictable if modeled carefully. Running into them in a sim is far cheaper than in a real factory.
• R&D needs ranking. We can build our own CNC machines, finishing mills, controllers and heat treat furnaces. The sim should tell us which of these bets most improves the best achievable business.
Assumptions baked in
• Design, CAM and machine control software is free and instant. Physical machines can be bought or built, but parts, assembly labor, space, energy, materials, logistics and people all cost real money and time.
• The company starts with venture capital and uses debt only in forms that exist in practice, such as equipment loans against resellable machines.
• Production is high-mix: many diverse parts and any single SKU is rare.
• Initial scope: CNC machining, mold making, injection molding, heat treatment, and shipping custom metal and plastic parts.
• Numbers are rule-of-thumb guesses for the first pass, kept in catalog data so they're easy to revise as better data arrives.
Uses
One model serves four uses, and the same code supports all of them.
Use
Who
What it gives us
Education
Founders, partners, prospective employees
Intuition for how a factory business really works, through hands-on play and debriefs
Strategy search
MCTS and solvers
The best achievable strategy for a given set of assumptions
R&D ranking
Founders
Which improvements most raise the best achievable cash flow, found by sweeping each lever
Digital twin (later)
The real company
Plans and what-ifs started from the real factory's current state, with models scored against real data
Before a factory exists, calibration comes from vendor quotes, published time standards, interviews with machinists and moldmakers, shop visits and bench tests. A "conventional shop" baseline must reproduce familiar industry figures (revenue per employee, utilization, margins, mold lead times) before any what-if is trusted.
Organization
The code is cut along the lines of the teams that will run the real company (Conway's law used on purpose), so the contract between two modules is the contract between two teams.
Four layers
Layer
What it is
Owner
Central player
Every business and investment decision, for one factory or all of them: an MCTS agent with a configurable loss function, or a human in the 2D game
The player
Physical models
Each team's vertical slice: dynamics and observation models (conditional, at several fidelity levels), its estimator slice, and its message schemas, which its real systems also use
The team
Protocols
One central standard protocol: a shared envelope, one schema language, a small shared vocabulary for facts and paperwork that cross team boundaries (work orders, purchase orders, maintenance tickets, NCRs, shipments, handling unit moves), and team-owned payload schemas
Shared by both sides; platform keeps the registry
Core
Clock, event log, world state and layout, material tracking record, ledger engine, random streams, pull execution rules, scoring harness
Platform team
Vertical teams. Each physical system has one team that builds it, maintains it, improves it and models it, and that team owns its full vertical slice of the codebase: its message schemas in the standard protocol, its real controllers reporting on that protocol, its dynamics model (the sim plant's truth and the controller's prediction model), its observation model, its slice of the state estimator, its projection onto shared facts, its cell menu with configuration bundles, its cell visualization, its human-readable cell status, its inspector description and its tests. See "Standard protocol and team vertical slices" under Software architecture. Teams don't make operating decisions; they answer for what happens when their system is used in a given way. When a team grows, split it by system (mills vs. lathes), never by function.
A pull-driven floor. Day-to-day operation is autonomous. Work is released at the bottleneck's pace (drum-buffer-rope), WIP caps limit queues, routing and setup sequencing are automatic, and incidents get standard responses. The player, human or agent, sets direction rather than steering workers.
Records are shared; policies are owned. The material tracking record and the ledger are shared infrastructure that every team writes to under rules. Reorder points and storage rules belong to Materials; traceability rules belong to Quality.
Boundary test. If two teams would need to meet daily to change how they hand off work, they belong in one module.
Multiple sites. Plant teams exist once per site; sales, finance, people policy and equipment R&D are shared across sites.
Modeling
Every team model is a conditional distribution of outcomes given conditions of use, swappable across fidelity levels behind a fixed contract.
Conditional models. Examples: downtime given usage hours and load, cycle time given part features and material, die change time given die size, scrap given tolerance and machine condition. The central player owns the conditions; teams own the distributions.
Fixed contracts, swappable fidelity. A module always answers the same question; only the method changes. Machining runs from fixed times, through removal rate and feature-based estimates, up to full CAM from STEP files. Truck arrivals run from a Poisson process up to routes and traffic. Scenario files choose the level for each module. Each module has its own random stream, so upgrading one doesn't reshuffle another's luck.
Detailed and coarse sims. The detailed sim plays out every walk, grasp, cycle and truck, using time standards (MTM or MOST) for people. The coarse sim runs on rates and delays calibrated from the detailed one, fast enough for search. High fidelity always calibrates low fidelity.
Outcomes, not just results. Every action can end completed, completed with defects, aborted, or as an incident. Incidents (fire, flood, crash, spill) share one format: type, severity, location, what's affected, cleanup tasks, cost and downtime. External hazards such as storms, outages and supplier disasters are their own module.
True vs. believed state. Modules track both the real value (coolant level, tool wear, inventory count) and what sensors and people believe it is. Mistakes come from acting on the wrong belief. These are beliefs of simulated people and machines inside the plant, distinct from the controller's estimated state (see Plant and controller). Workers notice problems as they walk past, with odds by skill and fatigue; nobody notices during unattended hours.
Material tracking. Every bin, bar or box records its form, packaging (loose, bagged, boxed, palletized), batch and origin, properties, location, and known vs. true state. Mixing or repacking creates new tracked units, so lost traceability (resin batches blended in a dryer) carries visible uncertainty.
High-mix production. SKU demand follows a long tail: a few steady sellers, many rare ones. That makes changeovers, setup sequencing, quick-change hardware (zero-point clamping, quick mold change, master unit dies), part families and first-article success major drivers of throughput.
Realism systems covered: tool wear per individual tool, coolant and chips, fixtures, heat treat recipes and failures, mold process windows found shot by shot, CMM inspection, breakdowns and spares, raw material sawing and offcuts, outside finishing, deburr and marking, unattended night runs, thermal drift, revision changes, absences and turnover, utilities and demand charges, shipping damage, emergency store runs.
Solving the factory
The factory is solved in layers: solvers decide what to build and where; search decides when to commit.
Layer
Question
Method
Capacity planning
Machines, people and buffers needed, given current models and failure rates
Stochastic programming, or sample-average approximation
Shadow prices
What a small improvement to each constraint is worth
Lagrange multipliers from the dual; the largest marks the bottleneck
Large hypotheticals
What a big change is worth, e.g. a custom CNC at a third of the price
Re-solve across the lever's range, then confirm in the sims
Layout
Where machines, aisles and storage go
PCB-style placement and routing
Variability check
Whether the plan survives randomness
Coarse sim, then detailed sim
The path
When to raise money, enter markets, lease, add capacity
MCTS, with the capacity model as a fast value estimate
• Correct for variability. Average-flow models treat 98% utilization as fine. Queueing approximations (Kingman-style), chance constraints or sim-calibrated margins fix that.
• Shadow prices are local. They value small improvements near the current plan and aren't clean for whole-number choices (whole machines, whole people). Big bets are re-solved.
• Layout design rules: aisle widths, forklift access and turning radius, floor loads, utility drops, crane coverage, hazmat zoning, fire egress. Objectives: short material flow, short operator travel, fewest operators, room for expansion. Moving an existing machine carries a large penalty for rigging, recommissioning and lost production.
• Layout feeds back. Walk distances and which operators can tend which machines change staffing, so the loop is size, lay out, measure, re-size.
• R&D ranking. Sweep each lever across its range, let the agent find the best strategy at each setting, and compare best achievable outcomes (cash after N years, peak cash need, time to break-even). Sweep pairs to find bets that pay off only together.
Gameplay
The game is 2D layout plus time management of commitments under a cash constraint: formally an event-driven, partially observable decision process (a semi-Markov POMDP).
State
Part
Contents
Observable?
Physical
Layout, machines, inventory, work in progress
Yes
Pipeline
Every commitment in flight (orders, permits, open roles, R&D programs) with lead-time distribution, money committed, payment schedule, cancel and expedite terms
Status yes, completion no
People
Staff, skills, assignments, ramp-up in current domain
Mostly
R&D
Per team target: current vs. target distribution, count or cost, staffing, budget, hidden progress
Metric yes, progress no
Market
Backlog, bids, customers, reputation, certifications
Mostly
Money
Cash, debt, credit, scheduled receivables and payables, equity, runway
Yes
Beliefs
Uncertainty over model parameters
It is the uncertainty
Moves
• Commit: order machines, stock or spares; apply for permits; post roles; lease space; sign financing; bid on work; buy certifications; fund marketing; task vertical teams with targets (a target conditional distribution, an integer count of machines, or a target cost), fund and staff them, and offer bonuses for hitting them.
• Adjust: expedite or cancel orders, reassign people, change R&D budgets, set floor policy (WIP caps, buffers, maintenance timing).
• Place: put a machine footprint on the floor, or move one.
• Wait: advance to the next event or a chosen date. Waiting buys information. This is the same skip-ahead action human players use (see Game client).
Actions need their prerequisites (a machine install needs delivery, leased space, a power drop and any permit), forming a dependency graph whose critical path is the time-management problem. Decisions happen on events (delivery, bid result, permit ruling, hire, R&D breakthrough, incident) and at a weekly review, never every tick.
Stochastic processes
Process
Model
Lead times
Distribution per item and vendor; late orders tend to stay late; shared shocks cluster delays; noisy ship-date updates; expediting costs money
Permits
Multi-stage, with pass, resubmit and denial odds; some parallel, some sequential
Hiring
Time to fill and offer acceptance from the labor market, then onboarding
Ramp-up
Productivity in a new domain rises roughly as 1 − e^(−t/τ), faster for adjacent skills; newcomers briefly slow the team
R&D
Each team target has a stochastic lead time and a distribution of outcomes (miss, hit, exceed) that depend on ramped staffing, budget, bonus size and how ambitious the target is, with diminishing returns. These start from baseline distributions, pooled across teams and updated as the company runs. Progress is hidden until delivery or a milestone
Reward. A configurable loss: company value at a date, cash after N years, or growth without running out of cash. Running out of cash with no credit ends the game. Every commitment writes its future cash flows into a projected cash curve, and runway is read off it.
Ways to play
Mode
How
Interactive 2D game
Humans see the floor plan and a timeline of everything in flight with uncertainty bands, dependencies and cash impact. The floor runs itself; players set direction. Speeds from pause to fast-forward; fork, save and rewind
Optimal play
Solvers propose candidate plans and placements as macro-actions; MCTS chooses among them and decides when to commit, using chance nodes or sampled futures
Assisted play
Buttons in the game: run MCTS from here, suggest a layout, show shadow prices
Group play
Everyone plays the same versioned scenarios; decisions are compared with each other and with the agent's. Disagreements expose hidden assumptions
Module details
Each team owns a few modules; each module exposes conditional models at increasing fidelity behind one contract.
Team
Modules
Key conditional models
Fidelity, simple → detailed
Machining
Machining process, its maintenance
Cycle time given part features and material; tool wear per tool given cuts; scrap given tolerances, machine condition and probing; downtime given usage
Fixed time → removal rate → feature-based → CAM from STEP
Toolroom
Mold build, mold maintenance
Build hours by machine given geometry and steel; trial loops given design; mold wear given shots and resin
Lookup table → geometry-driven estimate
Molding
Molding process, its maintenance
Cycle given wall thickness and resin; defects given settings vs. the hidden process window; changeover given mold size
Fixed cycle and scrap → physics-based cooling and process window
Heat treat
Heat treatment
Outcome (hardness, cracking, warp) given recipe, load and alloy
Fixed time and yield → recipe-sensitive model
Finishing, assembly, packing
Deburr and marking, assembly, packing
Time and error odds given part, bill of materials, packaging
Fixed time per unit → station and machine models
Quality
Inspection
Inspection time and detection odds given features and equipment
Fixed pass rate → feature-based CMM model
Materials and logistics
Inventory policy, material handling, procurement, logistics
Lead time given vendor and item; move time given route and load; truck arrival and damage given carrier
Fixed lead time, Poisson trucks → vendor reliability, routes and traffic
Facilities and EHS
Site and layout, utilities, permits, incidents, external hazards
Permit duration and outcome given process and jurisdiction; energy and demand peaks given load; incident odds given conditions
Flat costs and fixed tables → load profiles, LLM-read zoning, causal incidents
Planning and flow
Pull rules, process planning (routing), bottleneck detection, coarse sim
Queue and lead time given WIP and mix
Simple rules → optimized release and sequencing
People ops
People, labor market
Task time and error odds given skill and fatigue; time to fill and wage given role and area; quit odds given pay, overtime, incidents
Fixed times → MTM motion chains; fixed wages → job board data
Sales
Markets and demand
Demand and win odds given price, market and reputation; bursty arrivals
Fixed demand → stochastic, bursty, competitive
Finance
Finance
Loan terms and approval given collateral and history
Fixed rates → underwriting rules
Platform
Core, agent interface, prediction evaluation (fsim-eval)
—
—
Decisions already made
• Machining: setups and operations are always generated automatically; tool wear is tracked per individual tool; scrap comes from a statistical tolerance model; in-process probing lowers variation at the cost of probing time; routing belongs to process planning.
• Molding: each mold, resin and press has a hidden process window, found shot by shot with a swappable tuning approach; two-shot, overmolding and insert molding live in molding; packing is its own module; resin batches carry tags, and blending in a dryer creates a blend with less certain properties.
• People: morale is a simple quit risk; workers notice problems as they walk past; hiring lives in the labor market module; urgent tasks can interrupt work, at a cost in stop time, ramp-up and sometimes scrap.
• Heat treatment options: outside vendor, bought box furnace, bought vacuum furnace, or a home-built furnace (cheap, more scrap while tuning, hard to borrow against).
Flow, goals and costs
The central player's throughput plan gives each cell a smooth ramp, the pull rules keep the floor flowing toward it, and teams are goaled on model accuracy and on improving their systems.
Ramps. Set by the central player in each area's own units: parts per day through machining, shots per shift through molding, furnace loads per week through heat treat. Ramps are expressed as conditional distributions, like everything else, and planning them is the central player's job. That puts more complexity on the player and keeps the teams' contracts simple.
Buffer zones
Position vs. ramp
Floor behavior
Well ahead (green)
Fit in maintenance, mold trials, training and improvement work
On track (yellow)
Normal operations
Behind (red)
Volume first; expediting and overtime allowed
Smooth beats fast. Kingman's approximation: queue wait grows with utilization ÷ (1 − utilization) times the variability of arrivals and service. A cell running below capacity with tight output bounds can help the factory more than one running flat out, and the sim prices that variance reduction in dollars.
Bottlenecks move. The sim detects the current bottleneck and forecasts it a few weeks out, flagging near-bottlenecks. Changeovers cost factory throughput only at the bottleneck; elsewhere they're nearly free, so non-bottlenecks run small batches.
Team targets. The central player tasks each vertical team with targets stated in the same language as its models:
• A target conditional distribution, such as "time for a machine to process a part, given its features" or "die change time, given die size."
• Integer counts, such as a number of machines of a given type in service.
• A target cost, such as cost per machine for a new design.
Each machine or process type exposes a capability vector through its fixed contract: its conditional distributions (cycle time, changeover, failures, scrap, handoff time at its squares) plus plain numbers (machine cost, floor space, power draw, operator minutes per load). Any part of that vector can be a target. How the team gets there, whether a bar-fed mill or a better fixture, is the team's business; the sim only sees the new numbers. Two guardrails keep targets realistic: costs and lead times rise steeply as a target nears physical limits, and linked numbers move together along a frontier of what's achievable at once, so a faster machine may also be bigger or pricier.
Trade-off signs. Each team publishes a crude sign matrix over its capability vector, a standard format the planner can read. Each entry is +1, −1 or 0 for a pair of quantities: +1 means improving one tends to improve the other, −1 means improving one tends to worsen the other (pushing cycle time down tends to push machine cost up), and 0 means no expected link. Like covariances in portfolio optimization, these are guesstimates; only the sign is meant to be right. The planner uses them as hints, such as warning when a target pushes against a −1 link, never as hard constraints, since creative teams often break apparent trade-offs. They are not scored.
Each target comes with staffing and budget, and the team has a stochastic lead time to deliver, with some chance to miss, hit or exceed.
How teams are scored and rewarded
• Scored on their process models: how accurately their conditional distributions reflect reality, and how good the distributions themselves are (faster, cheaper, tighter, more reliable).
• Not scored on predicting their own success. Teams hit too few targets for their own hit probabilities to be measurable, so nobody grades those estimates.
• Rewarded with bonuses. Hitting a target pays the bonus attached to it.
Bonuses are a game move. The central player chooses how big a bonus to offer for each problem in the factory. Bigger bonuses motivate teams more, shortening lead times and raising the odds of a hit, at a cost to cash. Where to put bonus money is part of the strategy.
Response to bonuses starts from baseline distributions: lead time and miss, hit or exceed odds, given target ambition, staffing, budget and bonus size. These are company-level models, pooled across all teams' targets and updated as the company runs, since no single team generates enough data on its own.
Shared cost attribution. Labor is a step cost: savings appear only when a whole person or shift goes away.
Number
Computed as
Used for
Allocated cost
Shared costs split with Shapley values, more stable than time shares
Pricing, quoting, long-run decisions
Avoidable cost
The sim runs with and without a set of tasks and measures what disappears
Automation and staffing decisions
Time-share allocation creates an internal "Dutch disease": a process's share of an operator rises when others go lights-out, though it changed nothing. Avoidable cost turns that into a visible opportunity, such as "this process plus press 3 together free a whole operator." The same applies to forklifts, cranes, chillers and supervisors.
Real operations
Once a factory runs, real data flows in, scores the team models, and improves the agent, while a decision log scores the agent itself.
Data in. We write the control software on every machine, so controllers emit our own event schema (timestamp, source, entity, payload) directly over defined web APIs, with no MTConnect or OPC UA layer. Badge systems, accounting, carriers and suppliers come in through small adapters. Games start from "now" or "X days ago" using snapshots plus recent events; hidden variables like exact tool wear are sampled from estimates, and MCTS plans across several samples.
Location tracking. People and forklifts carry ultra-wideband (UWB) tags read by anchors on the walls, at about 10–30 cm accuracy and a few updates per second; Bluetooth tags are cheaper but only accurate to a few meters.
• Ingestion: position fixes arrive through the twin service and are compressed into trajectory segments and zone entries and exits, the same segment format the sim uses.
• Matching: an HMM-based map-matcher snaps noisy fixes onto the navigation layout to infer which squares were visited, dwell times and routes taken.
• Calibration: matched trips feed the people and logistics models (walking and driving speeds, dwell times, congestion delays), scored as conditional models like everything else.
• Live view: the ops view draws real positions with the game's renderer, since they are the same segments, and can overlay planned against actual routes and expected against actual congestion.
• Safety: proximity alerts when a forklift and a person get too close.
• Privacy policy: tracking employees is sensitive and in some places regulated (in California, employee data falls under the CCPA). The data calibrates processes, not individual performance; what feeds models is pooled and anonymized; raw traces are kept only briefly; and employees are told clearly what is collected and why.
Conditional model scoring
• Per event. For each observed pair of conditions and outcome, score the outcome under the model's distribution for those exact conditions, with a proper scoring rule (log score or CRPS). Upstream errors never reach a downstream team.
• Conditioning variables are part of the contract. Each model declares what it conditions on, and the real system must log those values; unobserved ones are averaged out and flagged.
• Censored data such as machines that haven't failed yet is scored with survival likelihoods.
• Honest uncertainty off the map. Models widen their uncertainty in regimes history never covered; the planner flags decisions leaning on them, which tells teams where to collect data.
• Tamper-evident. Raw readings go straight into the shared event log, and the platform team runs the scoring harness.
• System validation is separate. Full historical replay checks that the composed models reproduce throughput, lead times and cash; it validates the whole sim and the central player, not individual teams.
Prediction evaluation module (planned)
A platform-owned module, fsim-eval, compares each team's predictions with the data stream that actually arrives. It measures model accuracy in practice, and it sanity-checks the simulator and the whole data pipeline.
• Predictions are logged when made. Before each observation arrives, the estimator already computes a predictive distribution for it, given the belief and the conditions at that moment. That distribution is logged in a controller-side prediction log, alongside the plant message log. Longer-horizon forecasts, such as lead times and cycle times, are logged too and scored when they resolve.
• Prequential scoring. Each observation is scored against the prediction made just before it: proper scoring rules (log score, CRPS) per team, per model and per conditioning regime, over rolling windows.
• Calibration diagnostics: residual mean and variance, coverage of prediction intervals (does the 90% interval contain the outcome 90% of the time?) and PIT histograms (the probability integral transform, which is uniform when predictions are calibrated).
• Drift alerts when a team's scores or calibration degrade, routed to the team and, for large divergences, to operations.
Sim sanity check. When the plant is a simulator running the same models and parameters the controller uses, errors must be zero-mean with known variance, intervals must have their nominal coverage, and PIT histograms must be uniform. A failure points to a bug rather than a modeling question: a unit mismatch, a time offset, a mislogged conditioning variable, or an estimator error.
• In CI, matched sim runs with fixed seeds must pass these calibration tests.
• Detection power is tested too: deliberately mismatched plant parameters must produce a detectable bias of the expected size, proving the evaluator can catch real errors.
• Device-level parity runs (simulated machines feeding the real ingestion pipeline) use the same checks to validate ingestion and estimation end to end.
Views of the system
View
Purpose
Interactive game
Humans play: layout, commitments, timing
Optimal play
MCTS and solvers find the best strategy
Live data ingestion
Controllers and business systems feed the twin
Operating dashboards
Bottleneck, ramps, cost attribution, shadow prices, runway, model accuracy
Conditional model scoring
Teams' models scored against real outcomes
Model lifecycle
Versioned registry; a new version must beat the current one on held-out history; per-team leaderboard
Experiments and research
Scenarios, lever sweeps, hypotheticals, results database; every run reproducible from seed, model versions and scenario hash
Decision log
Agent recommendation, actual decision and overrides, predicted vs. actual outcome
Data quality and divergence alerts
Missing data, sensor faults, clock drift; reality drifting from forecast alerts operations, not just model owners
External data feeds
Listings, zoning, wages, material prices, carrier rates, marketplace jobs; each owned and scored
Education and assessment
Curated scenarios, tutorials, debriefs; scrubbed scenarios for candidates
Also: ledger reconciliation against the real books, and admin with roles, redaction, ITAR controls and an audit trail. Agents recommend; people approve. The twin never drives machines directly.
Software architecture
Four deployable pieces share one deterministic Rust core: the core crates, a game and MCTS server, a twin and ingestion service, and a TypeScript web app that only views and sends commands.
Real factory data flows one way, from controllers into the twin and on to games as read-only snapshots; every server embeds the same core.
• Core (Rust). A composed pure step (detailed below), step(state, buffers, inbox, now, rng) → StepOut, with caller-owned state in simple, deterministic data structures (standard `BTreeMap` and `Vec`, wrapped in `Arc` at module/entity boundaries with monotonic version numbers for fast diffing). Strict determinism: ordered maps, integer time (`SimTime` in milliseconds) and integer money (`Money` in tenth-cents, `$1.00 = 1_000`), one seeded random stream per module, no wall clock, a golden-replay test in CI. Crates: core, protocols, several small crates per team, plus capacity, layout, coarse-sim, mcts and scoring. Each module trait has a conformance suite every fidelity level must pass. Enum dispatch in the hot loop.
• Game and MCTS server. Rust (axum, tokio) embedding the core. Each game is an actor owning its state, clock and speed, streaming diffs over WebSocket at 10–30 Hz. Play, pause, speed, fork, save, rewind. MCTS runs as jobs on a separate worker pool with per-user compute budgets.
• Twin and ingestion service. Separate from games so load never drops factory data. Device authentication (mTLS), sequence numbers, idempotency keys, an append-only event store and periodic snapshots. Games read real state and never write to it; real changes go through the ops UI with permissions and an audit log.
• Accounts and data access. Everything is served only to signed-in users: the game, the API, models and assets. Accounts have player, operations and admin roles. Two-factor authentication (authenticator-app codes, plus WebAuthn passkeys or security keys) is added early in development, is required for operations and admin roles, and is re-checked before approving any command to the real factory. Starting a game from real state passes through a filter that redacts customer, price and ITAR-controlled data the player isn't cleared for.
• Web app (TypeScript). One app, three modes: game, ops and dashboards. It renders and sends commands; all simulation and metrics run on the server, so dashboards read the same over a game fork as over the real factory.
User accounts
People log in to the server that hosts the game with their own accounts; nothing is reachable without signing in.
• Invite-only. The root admin generates invite links: single-use, expiring links that each let one person create one account with an email and password. There is no public sign-up. The root admin account is created on first run from the command line.
• Sign-in: email and password, with passwords hashed using Argon2id and sessions in secure, HTTP-only, same-site cookies that expire. Users can sign out everywhere.
• Password reset by a single-use reset link the admin generates, so no email service is needed; emailed resets can come later.
• Two-factor authentication soon after the first playable release: authenticator-app codes and WebAuthn passkeys or security keys. Required for operations and admin roles.
• Alternative: if every user has a company Google Workspace or similar account, single sign-on through it gives two-factor authentication without building it ourselves.
• Roles: player, operations, admin.
• Protection: rate limiting and temporary lockout after repeated failed sign-ins, and an audit log of sign-ins and admin actions.
• Games belong to users. The setup screen lists the signed-in user's games and checkpoints. Sharing a game read-only, or as a copy, comes later.
• Per-user quotas: number of saved games, storage, and MCTS compute budget.
• Account lifecycle: admins can disable accounts; users can export their games, and deleted accounts have their data removed.
• Built from vetted libraries (for password hashing, TOTP and WebAuthn) in the Rust server, with accounts and the game index in a small SQLite database. No hand-rolled cryptography.
Hosting and files on disk
The first version runs on a single Linux or Mac machine: one Rust server process, a small SQLite database for users, and everything else as files in one data folder.
• One server binary serves the web app and the WebSocket API over HTTPS, either directly or behind a small reverse proxy such as Caddy. The same code builds on Linux and macOS, with no platform-specific dependencies.
• SQLite holds users, invites, sessions and the index of games and checkpoints. Nothing else lives in the database.
• Everything else is files, in formats chosen for each kind of data:
data/
  users.sqlite                 accounts, invites, sessions, game index
  catalogs/                    team menus, buildings, markets (human-editable text)
  assets/<hash>.glb            cell models, content-addressed
  games/<game-id>/
    game.json                  owner, scenario, seed, schema and model versions
    log/<segment>.log.zst      message log: framed envelope + payload records, hash-chained
    commands/<segment>.log.zst commands with approvals
    predictions/<segment>.zst  controller prediction log
    snapshots/<seq>.snap.zst   estimated-state snapshots, named by log position
    truth/<seq>.snap.zst       sim plant true state (sim games only)
    checkpoints.json           named and autosave checkpoints
• Formats: logs are append-only segments of length-prefixed binary records with a versioned header, compressed with zstd when a segment closes. Snapshots use the standard schema encoding, compressed. Metadata and catalogs are readable text (JSON or TOML) so people can inspect and edit them.
• Safe writes: files are written to a temporary name, flushed and renamed, so a crash never leaves a half-written snapshot. Open log segments are flushed on every record batch.
• Backups: copying the data folder is a complete backup. Replication to cloud storage comes later, file by file, since closed segments and snapshots never change.
Networking
To the game server, a sim plant looks exactly like the real factory, down to one authenticated session per reporting device. The sim gets there with an in-process virtual network instead of separate processes or OS-level virtual interfaces.
Real factory: one session per device
• Every reporting device has its own identity and client certificate: machine controllers, sensor hubs, badge readers, location-tag gateways and operator tablets.
• Devices connect outbound to the game server over mutually authenticated TLS, which avoids firewall and NAT problems at the plant; only the server needs a stable address.
• Hierarchical device ids, such as plant/7/cell/12/mill-controller. Each device has its own session, sequence numbers and clock.
• Commands flow back down the same sessions. Many go to people's devices (tablets, label printers), since people carry out most commands.
Sim: a plant host full of virtual devices
• The sim plant runs in a plant host that creates one virtual device per sensor or controller. Each team's slice declares which devices its cells have and what each reports.
• Each virtual device opens its own session with its own id and credentials, using the same handshake, framing and envelope as a real device. The gateway sees N authenticated device sessions either way.
• Sessions run over an in-memory virtual network with the same API and no sockets, in the spirit of the turmoil crate's deterministic in-process network simulation.
• Why not a child process or a virtual network interface: TUN/TAP interfaces need root and differ between Linux and macOS, and a separate process makes determinism hard because both sides must run in lockstep on sim time. The in-process virtual network gives the same protocol-level parity, runs fast and stays deterministic.
Transports behind one interface
Transport
Used for
In-memory virtual network
All normal sim games; fast and deterministic
Loopback with real mTLS, plant host as a separate process
CI integration tests of the real network stack and gateway
Real network
Real factory devices
Simulated network trouble. The virtual network can inject latency, jitter, dropped messages, disconnects and clock skew per device, seeded and deterministic, so the gateway, message log and state estimator are exercised against the problems a real plant will have.
Sim control channel. Time jumps, run speed, pause, fork, snapshot and restore, the god view and fault injection go through one sim control channel per sim plant, never through device sessions. The server opens it only for plants of kind sim; real plants have no such endpoint, so these commands can never reach real-factory code paths. MCTS rollouts never use any of this; they run on the in-process model API.
Time. The server never reads the wall clock directly; it uses a clock abstraction. In sim games time comes from the sim control channel, and skip-ahead asks the plant host to advance to the next event, with messages stamped in sim time. For the real factory the clock is the wall clock.
Crate
Owns
fsim-net
Device-session trait, device directory, transports (memory, mTLS, simulated network with impairments)
fsim-gateway
Accepting sessions, authentication, rate limits, writing the message log, feeding the estimator
fsim-plant-host
Running a sim plant, creating virtual devices from team declarations, the sim control channel
Plant and controller
The system has two sides separated by one API. The plant (the real factory, or a simulator standing in for it) holds the true state. The controller (GUI, dashboards and agent) holds only an estimate. Truth never crosses the boundary; only observations and commands do.
Term
Lives on
What it is
True state
Plant only
What is actually happening: real tool wear, real coolant level, real hidden progress
Observations
Cross the boundary
Sensor readings, badge and location scans, controller events, human reports, delivery confirmations, command acknowledgements. Noisy, delayed, sometimes missing or wrong
Estimated state (belief)
Controller only
A probability distribution over the true state, built from observations by the state estimator
Model
Controller only
The same sim crates used for prediction, with the controller's estimated parameters, which may differ from the plant's true ones
Commands
Cross the boundary
Orders, placements, policies and assignments with effective times; executed by the plant with latency and possible failure
The plant API. Both plant kinds implement one contract:
• Observe: a timestamped stream of observations in our own event schema.
• Command: send commands with an id and effective time; acknowledgements come back asynchronously, as observations that can themselves be late or wrong.
• Capabilities: a sim plant also offers fork, save, rewind and speed; the real factory declines them.
The GUI, dashboards, decision log and the agent's outer loop use only this API, so they are written once and never know which kind of plant they're driving.
The controller side
• Only the estimate is ever shown. The GUI draws estimated state, because in the real factory that's all anyone can know. It shows uncertainty where it matters (coolant "about 40%, ±10%", tool life "30–60 minutes left") and lets stale information fade.
• The state estimator turns observations into belief, using two conditional models per module: a process model (how state evolves) and an observation model (what sensors and people report, given the truth). Teams own both, since they write their systems' controllers and sensors. Fidelity ladder: treat observations as exact, then per-variable filters (e.g. Kalman filters for continuous levels, counts for inventory), then a particle filter over joint state.
• MCTS plans on the controller side, inside the belief. At each decision point it samples several complete states from the belief, runs forked rollouts of the model on each, and aggregates. That is POMDP planning by sampling (in the spirit of POMCP). It never sees the plant's truth.
• Model mismatch is testable. A sim plant can run on "true" parameters the controller doesn't know, while the controller plans with its own estimates, which is exactly the situation in the real factory.
• The estimator is deterministic (seeded) like everything else, so sessions replay exactly.
Forking and rewinding
• Against a sim plant, a fork copies both sides: the plant's true state and the controller's belief.
• Against the real factory, a fork creates a new sim plant whose true state is a sample from the current belief (the truth is unknown), paired with a copy of the belief. "Play from now" means exactly this.
Truth for debugging and teaching. A separate "god view" channel reads the sim plant's true state for developers and for debriefs ("here's what was really happening"). It is clearly marked, never available for the real factory, and never used by the agent.
Parity depths
Level
What matches
Use
A. Plant API
Commands and observations
All games and agent tests
B. Device level
Simulated machines emit controller events into the real ingestion pipeline
CI integration tests; estimator calibration with injected noise, dropouts and clock skew
C. Hardware in the loop
A real machine controller drives a simulated machine
Controller development before machines exist
Costs. Rollouts stay in-process on the controller side, never over the network. From the very first playable version, the sim plant runs in `fsim-plant-host` (in-process, no separate OS processes) and communicates with `fsim-gateway` over the `fsim-net` in-memory transport using the exact same authenticated device-session API and Protobuf envelopes that real machines use. The estimator can start as "observations are exact," so the full network and estimation boundary exists from day one. The plant API is the most important contract in the system and is versioned from the start. Real plants have a distinct identity and credentials, and commands to them go through the approve-before-acting gate, so a game can never drive the real factory by accident.
Standard protocol and team vertical slices
One central standard protocol carries everything, and each team owns its full vertical slice inside it: its physical systems' reporting, its slice of the plant sim, and its slice of the state estimator.
The standard protocol (owned by the platform)
• One envelope for every message: team tag, schema id and version, source device, entity id, plant timestamp, receive timestamp, sequence number, correlation id (the command or job it relates to), payload.
• One schema language and encoding for all payloads, chosen centrally: Protocol Buffers (Protobuf via `prost` in Rust and `ts-proto` in TypeScript), so every team's messages are defined, versioned, validated and decoded with full forward and backward wire compatibility (old logs decode forever, and floor machines can upgrade independently without lock-step deploys). A central registry lists every schema and version.
• A small shared vocabulary for facts that cross team boundaries: handling unit moved to a square, machine state changed, resource taken or released, ledger entry, incident, command acknowledgement. Changes need agreement from the teams on both sides.
• Shared commands (placements, orders) alongside team commands ("set this press's injection profile").
The core routes, logs, orders and replays messages using only the envelope, never the payload contents.
Each team's vertical slice
Piece
Role
Message schemas
The team's observations and commands in the standard schema language, every version kept
Real systems' reporting
The team's machine controllers emit those messages directly
Dynamics model
How its systems evolve. One implementation serves twice: as the sim plant's truth, and as the controller's model for MCTS rollouts, run with estimated parameters on sampled states
Observation model
What its sensors and people report given the truth. The sim plant uses it to generate messages; the estimator uses it to weigh evidence
Estimator slice
Updates the team's part of the belief from its messages
Projection to shared facts
Reports what happens at the team's edges in the shared vocabulary
Inspector description
Fields, units and distributions for the GUI to render generically
Tests
Conformance suite, decoder fuzzing, calibration and scoring
Each slice also includes the team's cell menu with configuration bundles, and a pure visualize function that draws any of its cells, at any configuration and version, from estimated state (see Game client).
Three composed functions with one shape. The plant step (truth), the model step (prediction, the same code with estimated parameters) and the estimator update (belief) are each composed from team slices, the way the step function already is. Cross-team coupling, such as where a handling unit is, goes through shared facts held by the core.
Requirements that come with the freedom
• Decoders never panic on malformed bytes; they return errors, are fuzz tested, and ingestion quarantines bad messages.
• Old data decodes forever. Teams keep every schema version they've shipped, and CI replays archived real and sim logs to prove it.
• Sim and real speak identical bytes, because the team's sim plant and its real controllers use the same schemas. Device-level parity follows almost for free.
Storage and audit
We keep a complete, auditable log of every message that crosses the plant boundary, plus periodic snapshots of the controller's estimated state; together they reproduce any past belief exactly.
The message log
• Everything that crosses the boundary: every observation and acknowledgement the plant sends, and every command the controller sends, with who approved it.
• Stored as received: envelope plus raw payload bytes, decoded only when read, so a decoder bug never corrupts the record.
• Append-only and tamper-evident: sequence numbers, a hash chain across entries, write-once storage, and off-site backups.
Estimated-state snapshots
• Periodic snapshots of the controller's best estimate of the factory, for example hourly and at every shift change.
• Each snapshot records the log position it reflects, every team's schema and estimator version, and the estimator's random seed.
• Exact replay: loading a snapshot and replaying the log from its position through the same deterministic estimator reproduces the belief at any later moment.
Team responsibilities
• Every team's estimator slice serializes and deserializes its part of the belief, in the standard schema language.
• Round-trip tests: deserializing a serialized belief gives back exactly the same belief, fuzz tested.
• Old snapshots load forever, directly or through migrations, the same rule as messages, cell configurations and assets.
• Size budgets: large beliefs, such as particle sets, are stored compactly, for example as summaries plus seeds, within a per-team size budget.
What this enables
• Starting a game from "now" or "X days ago": load the nearest snapshot and replay to the chosen moment.
• Rescoring history: replay the full log through an improved estimator or model to see how it would have done.
• Audits and the decision log: every recommendation, approval and outcome links to the exact belief and messages behind it.
Separate stores. Sim plants also save true-state snapshots for fork, rewind and the god view, in a separate store that is never mixed with real factory data.
Retention exceptions. Raw location fixes are kept only briefly under the tracking privacy policy; the compressed trajectory segments stay. Logs containing customer or ITAR-controlled data live in access-controlled storage.
Composed pure step
Nearly all code lives in one pure step function, built by recursively composing pure step functions from each vertical team's crates. Servers, ingestion and I/O are a thin imperative shell around it.
// Same shape at every level: factory, team, cell, machine, tool.
fn step(
    state: &LocalState,         // this module's own state only (persistent, structurally shared)
    buffers: &BufferView,       // live contents of the buffers it pulls from and puts to
    inbox: &[Msg],              // information messages addressed to it
    now: SimTime,               // current time; the module integrates from its own last-update time
    rng: StreamRng,             // split per module and entity from (seed, id, event count)
) -> StepOut {
    // new state (or the same state, unchanged),
    // buffer ops: items taken and items put (always valid: the module saw the live buffers),
    // outbox messages, outcomes for the log,
    // next_event_at: when this module next expects something to happen
}
The contract for a team crate
1.  Declare its buffers: the inputs it pulls from and the outputs it puts to, by item spec.
2.  Implement step: own state in; new state, buffer ops, messages, outcomes and next event out.
3.  Pass the conformance suite: conservation, determinism, the early-call invariant, and calibration against a higher fidelity level.
Inside that contract a team crate can do anything, and so can the central planner, through priorities, buffer capacities, release rules and its other moves.
Two channels
• Messages carry information: backward-chaining internal orders, work orders, NCRs, purchase orders, task dispatches, alerts. Protocol types from fsim-protocols.
• Buffers carry things. A buffer is a physical place on the floor (a kanban square, rack, conveyor or dock lane) with a location, a capacity and contents. The core owns buffers. A module puts finished items into its output buffers and takes items from the buffers it consumes.
This gives pull for free: a module can finish a part only if its output buffer has room, downstream modules pick up parts when they have capacity, and buffer capacity is the WIP cap, drawn as a real square on the 2D floor.
People and iPads (`fsim-people`)
• People are managed by their own vertical-slice module (`fsim-people`) rather than pulled passively from a buffer. Each worker carries an iPad (a virtual device on `fsim-net`) and is directed by a central dispatch algorithm owned by the People module.
• Workers cannot be interrupted mid-task. When a worker finishes a task, they consult their iPad for the next dispatched task; both communication/dispatch handshakes and physical tasks take a stochastic amount of time.
Backward-chaining orders (recursive pull chain)
• Orders propagate backward up the production chain. When a customer order arrives, it creates a Shipping Order at the shipping bay for the finished part.
• If Shipping does not have the packed item, it sends a blocking upstream order to Packing. Packing sends blocking upstream orders for both the cardboard box (to Storage/Receiving) and the finished part (to Inspection or CNC). CNC sends a blocking upstream order for cut-to-size stock (to the Saw, or precut stock in the tutorial). The Saw sends a blocking upstream order for raw bar stock (to Storage/Receiving, which triggers a vendor Purchase Order if not on hand).
• Every order except the leaf orders at the start of the available material chain is blocked by its upstream dependencies. Unblocked orders are fulfilled in any order, prioritized by the People and Planning dispatch algorithms.
• Each unblocked order cleanly decomposes into two phases: (1) a logistics phase ("bring the required input handling units from their upstream output squares to my named input squares") and (2) a work-cell phase ("process inputs into the output square and mark the order fulfilled, unblocking the downstream order").
First-come-first-served passes
At each event time the core calls modules one at a time. Each sees the live buffers and takes what is there, so two modules can never take the same part and nothing needs arbitration.
• Order is an input. OrderPolicy::Forced(list) fixes the order for tests, so expected results can be asserted exactly. OrderPolicy::Seeded(seed), the default at runtime, sorts modules by a hash of the seed, timestamp and module id: unbiased, so no module is systematically favored, yet fully reproducible, and stable when a module is added.
• Never unseeded. "Random at runtime" means a seed chosen when a game starts and recorded with it, so replay, rewind, forks and MCTS all reproduce exactly. The core never reads OS entropy.
• Priority first. Modules the player marks as priority are called before the rest, which covers hot jobs and contested people and forklifts.
• Repeat until quiet. The core runs further passes at the same timestamp until a pass moves nothing, so a part put by molding can be picked up by packing in the same instant whatever the order. Passes always end, since each uses up items or capacity; a cap catches bugs.
• True ties are rare. In continuous time two modules seldom act at the same instant; ties come mostly from batched events, where an unbiased tie-break is the right answer.
Layout, buffers and logistics
Layout is pure data, physical handoffs happen at standardized buffers, and moving things between them is split between fixed transport placed in the layout and freeform logistics that backfills everything else.
• Layout is pure data. Footprints, buffer locations, aisles, doors and zones. The player edits it; Facilities owns the rules.
• One geometry layer. A small stateless crate (fsim-layout-graph) derives a transport network from the layout and installed transport: a graph whose edges are walking paths, forklift aisles and fixed links, each with a length and capacity. Results are cached by a hash of the layout; the reference is exact shortest paths. No team crate reads the floor plan directly.
• Owned buffers. Each team's input and output buffers sit inside its footprint and count toward its floor space. A team only puts to and takes from its own buffers; those buffers are its physical boundary.
• Shared buffers (staging, warehouse, dock lanes) sit elsewhere and belong to freeform logistics.
Standard interface: handoff squares
Each cell declares one or more named input squares (for example, `part_in` and `consumable_in` on a packing station) and a fixed output square, each holding at most one handling unit. The squares are the whole physical interface; how anything gets on or off them is someone else's problem.
• Size-one buffers. Each named input square holds one handling unit, whatever it is (a bare blank, a tray of parts, a cardboard carton, a fixture pallet, a gaylord of pellets, a mold). The output square holds one handling unit. A cell can start work only when all required named input squares for the order are filled, and can finish only when its output square is empty, which gives pull with the tightest possible WIP cap.
• The cell serves its own squares. Moving an item from the input square into the machine, and from the machine onto the output square, is the cell's responsibility. A cell may do it with a person, a robot arm, a gripper on the machine, or anything else, as part of its own module and model.
• Anything can serve a square from outside. A person, cart, forklift, mobile robot or belt can fill an input square or empty an output square. A conveyor simply ends at one square and starts at another.
• Fixed geometry. Each square has a fixed position in the cell's footprint and a size, so the layout and transport graph know exactly where every handoff happens.
In the sim, a square records its position, size, the item it holds (if any) and which movers can reach it. Every physical handoff between modules happens at a square.
Handling units and conversion
Everything moves as a handling unit: a container plus its contents (a tray of 24 parts, a fixture pallet with one clamped blank, a gaylord of molded parts) or a bare part with no container. Handling units nest, using the same structure as the material tracking record.
• Container catalog. Trays, totes, pallets, fixture pallets, drawers and gaylords, each with dimensions, capacity, weight, cost and stackability.
• Square specs. Each machine type declares which handling units its input square accepts and which its output square produces, such as "bare part up to 5 kg," "Tray-A, up to 24 parts" or "fixture pallet P2." This is part of the capability vector, so "accept trays" is an ordinary R&D target.
• Packing specs. Each part states how many fit in each container type, derived from its bounding box or listed.
• Transfer batch size. Each job sets how many parts travel together; the cell decides internally whether to finish the whole tray before releasing it.
• Conversion stations. Palletizing, depalletizing, kitting and unloading trays into bare parts happen at stations: modules with an input square, an output square and a conditional model of time and errors per part. The fixed transport team owns them, and the player places them in the layout.
• Moves never change contents. A move carries a handling unit unchanged. A person who carries a tray over and unloads it by hand is doing a move and a conversion, and both are charged.
• Process planning enforces compatibility. Every hop in a routing either joins compatible squares or passes through a conversion station; otherwise the routing is invalid. Conversion costs show the planner what "accept trays" R&D would be worth.
• Empty containers flow back. Trays, totes and fixture pallets are reusable assets that must return to where they're loaded. Container pools belong to freeform logistics, their size is a player decision, and running short starves cells.
• WIP limits count handling units. A tray fills a whole square, so more parts per tray means fewer moves but longer lead times.
Three kinds of transport
Kind
Examples
Owner
How it enters the sim
Within-cell automation
Whatever moves items between the cell's squares and its machine: a person, robot arm or machine gripper
The cell's own team
Part of that team's module
Fixed transport between cells
Conveyors running from one cell's output square to another's input square; gantry cranes for heavy items like molds
Fixed transport team
Placed by the player during layout. Each link is a module with an input buffer, an output buffer and a transit time
Freeform logistics
People, carts, forklifts, mobile robots
Freeform logistics team
Serves any move no fixed link covers. Also owns shared buffers
• Routing rule. If a fixed link joins two buffers, the item goes on it; otherwise the move becomes a freeform job. Pull is capacity-based (CONWIP, or POLCA where routings vary): when a square frees up, the next job is released by a dispatch policy the central player sets, such as due date, grouping jobs to save setups, or priority.
• Freeform logistics is the backfill. It starts with people and later adds carts, forklifts and mobile robots. Its conditional model gives move time, cost and damage odds given route features (distance, modes, load, congestion), so layout reaches the sim only through the transport graph.
• Congestion fidelity: distance ÷ speed, then aisles as shared capacity, then full multi-agent path planning, all behind the same contract.
• Layout pays off visibly. Moving a machine changes the graph and freeform logistics' costs, and adding a fixed link removes freeform jobs. The layout solver optimizes directly against these models.
Thing
Owner
Floor plan, zones, layout rules
Facilities (the player edits)
Transport graph
Pure crate derived from layout
A team's input and output buffers
That team
Fixed links between cells
Fixed transport team
Shared buffers, mobile movers, move jobs
Freeform logistics team
What moves when
Freed squares, plus the player's dispatch policy and priorities
Time advance rules
• Each module reports its own next event. The core jumps the clock to the earliest next_event_at, capped at the next UI frame, shift end or decision point. There is no central event queue; scheduling stays inside each module. A team step reports the earliest event among its sub-steps.
• Sample once, store it. A module samples a random finish time once and stores it; being called early never resamples it. If conditions change mid-cycle, it resamples only the remaining time, from the conditional distribution given time elapsed.
• Continuous quantities update lazily. Coolant, tool wear and resin moisture are integrated from the last update to now whenever the module is called; their next event is a threshold crossing.
• Early-call invariant. Calling a module at an extra time with no new input must not change any of its future outputs. This is an exact test in every conformance suite.
• Reference semantics. The simple reference loop calls every module in every pass; a module with nothing to do returns its state unchanged. Skipping idle modules is a later optimization, and must match the reference exactly under fuzz tests.
• Coarse mode uses the same contract with fewer, aggregated events calibrated against the detailed sim. Viewing interpolates stored start and end positions, so the sim never steps just for animation.
Invariants and tests
• Conservation: every physical item is in exactly one place (a buffer, a module's state, or in transit), checked every step. Inventory value moves through the ledger with it.
• Determinism: same seed and inputs give bit-identical output, under both order policies.
• Forced-order tests assert exact results; shuffle-seed sweeps check that aggregate statistics don't depend on the seed.
Engineering conventions (full text in AGENTS.md):
• Many small, stateless crates with small APIs; state kept in as few places as possible (game actors and the twin's store).
• TDD; fuzz tests behind --features fuzz-tests; every optimized path has a simple single-threaded reference oracle in src/reference.rs to fuzz against.
• Correct-in-all-cases algorithms over edge-case accumulation, especially in layout and path-planning geometry.
• Optimize only by measurement: profile, find the bottleneck, one hypothesis at a time, revert what doesn't pay, and log it (OPTIMIZATION_LOG.txt, ML_LOG.txt, CALIBRATION_LOG.txt).
• One standardized README per crate as the agents' map of the codebase; no central architecture doc, no duplicated types or algorithms.
• Lower fidelity levels and the coarse sim are checked by statistical calibration tests, not exact-match fuzzing; only reference oracles are matched exactly.
• Frontend: one OOP state class mirroring server state plus UI-local state, pure tested helpers, useState for cosmetics only, and no simulation in the browser.
Movement and pathfinding
Every trip is planned once and stored as timed motion segments; the sim jumps between segment ends and the viewer interpolates, so movement is full fidelity without per-frame stepping.
People
• Navigation grid. The layout is rasterized into an occupancy grid at about 10 cm per cell, with obstacles inflated by body radius using a distance transform, so every free cell is passable.
• Search. A* with an octile heuristic, then Theta* (any-angle A*) to smooth zigzags into straight runs. Plain Dijkstra on the same grid is the reference oracle. A navmesh can replace the grid later if profiling demands it.
• Speed comes from the people model: about 1.2–1.4 m/s walking, slower when carrying, varying by person and fatigue.
Forklifts
• Hybrid A.* States are position plus heading; moves are arcs at the minimum turning radius, straight runs and reversing. The heuristic is the larger of the obstacle-free Reeds–Shepp distance (shortest path for a car-like vehicle that can reverse) and the grid distance that respects obstacles.
• Goals are poses, such as "square 12, facing north, forks centered." Near the goal the planner tries an exact Reeds–Shepp curve into it.
• Loads change the footprint. A carried pallet enlarges the swept shape, so aisle widths matter for real.
• Feasibility is guaranteed; optimality is not claimed. A separate exact validator checks every returned path: curvature never exceeds 1 ÷ turning radius, and the swept footprint never touches an obstacle. It samples poses along the path with the footprint inflated by the most it can move between samples, so a collision cannot slip between samples. Fuzz tests on random layouts check that the planner never returns a path the validator rejects.
Traffic (a fidelity ladder behind one contract)
1.  No interaction: each agent plans alone.
2.  Space–time reservations (cooperative A*): each trip books grid cells over time and later planners route around them. Deterministic, and congestion emerges naturally.
3.  Rules of the road: pedestrian walkways, forklift aisles, speed limits, yielding at crossings.
4.  Conflict-based search for small forklift fleets, if needed.
When something unexpectedly blocks an aisle, such as a dropped pallet or a machine being installed, only the affected agents replan the remainder of their trips.
Fit with the step contract
• A plan is a list of line and arc segments, each with a start time, a timing profile (acceleration, cruise, slowing for turns) and heading. Segments live in the mover's state.
• next_event_at is the arrival time or the next reservation check, never an animation frame.
• Paths are cached by layout hash, start pose, goal pose and load, in a caller-owned cache that layout changes invalidate.
• Coarse mode replaces planning with a table of travel-time distributions between squares, calibrated from the detailed planner.
Rendering. The viewer calls a pure pose_at(segments, t) for every mover each frame, so forklifts rotate through arcs with loads on their forks. The Rust motion crate is compiled to WebAssembly for this, so the arc math exists in one place only. Paths can be drawn as faint lines for debugging and spotting congestion.
Crate
Owns
fsim-geom
Exact polygon operations and footprint sweeps
fsim-navgrid
Occupancy grid, inflation, distance transform
fsim-path-walk
A* and Theta* for people; Dijkstra reference
fsim-path-vehicle
Hybrid A*, Reeds–Shepp curves, feasibility validator
fsim-traffic
Space–time reservations, rules of the road
fsim-motion
Segments, timing profiles, pose_at; compiled to WebAssembly for the viewer
Game client
The client is a low-fidelity 3D top-down view of the controller's estimated state, where teams draw their own cells, the core draws everything else, and players can plan changes freely before executing them.
Cell menus and configuration bundles (team-owned)
• Each team publishes a menu of work cell types it can install, such as a 3-axis mill cell, a 5-axis cell or a finishing mill cell for Machining.
• Each cell type has add-on and configuration bundles that change its price, lead time, footprint, utilities, staffing and capability vector. Examples: bar feeder, probing, pallet changer, larger tool changer or chip conveyor on a mill; robot picker, dryer, chiller or hot runner controller on a press.
• Every installed cell records its type, configuration and the team schema version it was created under. The team's crate must interpret state, models and estimates for every type, configuration and version it has ever shipped, the same "decode forever" rule as its messages.
• Prices and lead times come from the team's models, conditional on configuration, so they're also R&D targets.
Planning mode (GUI only)
• Placing a cell from the menu creates a planned cell, drawn as a ghost. Nothing is ordered and nothing happens in the plant. The player can move it, rotate it and change its configuration freely.
• A plan collects planned changes: new cells, moves, removals, configuration changes, plus the orders, leases and hires they imply. The plan shows validity (clearances, aisles, utilities, permits), total cost, lead time ranges and the effect on the projected cash curve and runway.
• Plans are controller-side objects, saved per game, never part of the plant. Players can keep several alternatives and compare them.
• Executing a plan sends its commands to the plant. Each command can still fail or run late, and progress shows on the pipeline timeline. Against the real factory, execution goes through the approve-before-acting gate.
• MCTS has no planning mode. It commits moves directly through the same commands.
Screen layout: tabs plus one detail view
• Tabs, not split views. The main area switches between tabs: Floor (the 3D view) and Timeline (the waterfall of everything in flight, with uncertainty bands, dependencies and the projected cash curve), plus Plants, Markets, Ledger and People, all planned from the start (described below). The top bar (clock, speed, cash, runway, alerts) stays visible on every tab.
• One right-hand detail view shows whatever is selected, on any tab, and is reused across a cell's whole life, so the player always looks in the same place.
Tab
Contents
Scope
Plants
Find, lease or buy space; track keys, utilities and build-out; pick which plant the other tabs show
Company
Markets
Unlock markets, set bidding policies, hire salespeople per market, watch order streams and performance (below)
Selected plant, with company-wide certs shared
Floor
The 3D top-down view, planning mode, overlays
Selected plant
Timeline
Waterfall of everything in flight with uncertainty bands, dependencies and build checklists as grouped bars
Selected plant, or all plants
Ledger
Money and dashboards (below)
Company totals, filterable by plant
People
Plant workers: staffing, hiring, shifts and priorities (below)
Company roster, filterable by plant
Ledger tab
• Statements from the double-entry ledger: profit and loss, balance sheet and cash flow, by month or custom range. Every entry drills down to the event, cell, job or order behind it.
• Cash projection: a fan chart of projected cash built from samples of the estimated state, with runway shown as a range (10th, 50th and 90th percentile) and the ruin point marked.
• Burn rate: gross and net monthly burn, split by category (payroll, rent, materials, equipment payments, outside services), with trends.
• Receivables and payables: aging, upcoming payments in and out, marketplace payouts.
• Financing: loans, credit lines, covenants and their headroom, equity.
• Unit economics: cost and margin per product or job, with allocated vs. avoidable cost.
• Dashboards: throughput against ramps, utilization, bottleneck location and forecast, shadow prices, team targets and bonuses, and model accuracy.
People tab
• Roster: each person's role, skills, wage, shift, current assignment, ramp-up level in their current domain, and quit risk.
• Hiring: post an open role with a wage offer and requirements; it becomes a pipeline item with a time-to-fill range from the labor market model, then candidates and offers, then onboarding.
• Shifts: shift patterns, coverage against demand by cell, overtime and its cost.
• Assignments: people to cells, teams or team targets, with a warning showing the ramp-up cost of a move.
• Task priorities: the order in which people pick up task types, such as machine alarms first, then loading and unloading, then deburring, then cleaning, set per shift or per role. These feed the pull rules.
• Training and costs: training plans that shorten ramp-up, and payroll with taxes, benefits and workers' comp.
• Real factory mode follows the tracking privacy policy: schedules and assignments are shown, but individual performance scores from location data are not.
Game setup screen
The first screen, before any game is open:
• New game: choose a scenario (starting cash and investors, region and market, realism settings), a random seed (shown and editable, so games can be shared and replayed) and whether to auto-pause at decision points.
• Load a checkpoint: saved games and autosaves, each showing the sim date, cash, runway and number of plants.
• Live factory: shown but disabled until the real-plant connection exists.
Tutorial, then sandbox. A new game offers a short tutorial, which players can skip at any point:
1.  Sign the lease on a first plant.
2.  Place one CNC cell and one packing station.
3.  Hire one employee.
4.  Open the Markets tab, turn on the online prototyping marketplace, and pick a middle-of-the-road price policy.
5.  Watch the factory come alive: jobs arrive, material is delivered, parts are made, packed and shipped, and money comes in.
To keep the tutorial this short, the default supplier delivers blanks precut to size, so no saw is needed yet. After that the game is an open sandbox: the objective is simply to make as much money as possible, in whatever way the player chooses.
Skip ahead to the next event. Long waits are common (a month for the keys to a new lease, two weeks before a new hire starts), so both players and MCTS can jump straight to the next thing that happens.
• What counts as an event: a pipeline item completing (keys, utilities, delivery, install, hire start, cert), an order arriving or a bid result, a cell blocked or down, an incident, a cash or runway threshold, and any decision-inbox item. Players can filter which kinds stop a skip.
• Skip variants: to the next event, to a chosen date, or until a selected pipeline item completes (picked on the Timeline tab; since its timing is uncertain, the skip runs until it actually happens).
• Interrupts: a skip stops early on incidents, cash warnings and anything the player marked urgent.
• The same action for MCTS. "Wait until the next decision event" is the agent's wait move, so the tree branches on decisions, not on idle time.
• Fast and exact. A skip runs the sim at full speed without rendering. Because the sim is event-driven, an idle stretch passes almost instantly. Skipping must give exactly the same result as playing through at normal speed, and a determinism test checks this.
Runway animation after a skip. When a player skips, a short animation (a few seconds, skippable) sweeps the projected cash and runway curve from the start of the skipped stretch to its end, with markers for what happened along the way (rent, payroll, deliveries, payments received). It ends on a summary card, for example: "Skipped 31 days. Cash $412k → $318k. Runway 9.1 → 7.6 months. 4 events," listing those events with links to each.
Games don't end unless the company runs out of cash and credit; otherwise play continues indefinitely.
Game analysis (later, not in the first playable release). Because every game keeps its full message log, commands and snapshots, any game can be analyzed after the fact:
• Replay any stretch at any speed, with the decision timeline alongside.
• Key metrics over time: cash, runway, throughput, utilization, bottleneck location, on-time delivery, quality and reputation by market.
• Decision review: each major decision with what was known then (the estimated state) and what happened next.
• Debrief with the god view: for sim plants, show what was really happening behind the estimates.
• Compare forks, other players' games on the same scenario and seed, or the agent's play from the same checkpoint.
• What-ifs: fork from any past checkpoint and try a different decision.
Plants tab
A game can run several plants, and the Plants tab manages them.
• Finding space: a browser of available sites using the building model (big warehouses, urban buildings, shipping container units), each with rent or price, size, clear height, doors, power and zoning.
• Getting into a space is a pipeline: signing a lease or purchase starts a lead time to get the keys, then utility setup (power, air, water, internet) and any build-out, each with its own lead-time range and cost. A plant becomes usable in stages, and its checklist appears in the detail view and on the Timeline tab.
• Plant selector: once there is at least one plant, the player picks which plant to view, and Floor, Timeline and the plant filters on Ledger and People update to match.
• A fresh game starts with no plants, so it opens on the Plants tab.
• Moving things between plants is a shipment with freight cost and lead time; people are hired into one plant's local labor market and assigned to plants.
Markets tab
The player doesn't bid on individual jobs. They choose which markets each plant enters, set a bidding policy per market, and orders arrive stochastically according to that policy.
• Per plant, with shared certs. Each plant has its own markets because certifications are mostly site-specific (quality-system registrations such as ISO 9001, AS9100 and ISO 13485 are typically issued per site). Some are company-wide, such as ITAR registration. The tab shows which certs each plant holds, which are company-wide, and which a market needs.
• Unlocking a market costs time and money: certifications (preparation, audits, months of lead time and ongoing surveillance audits), required equipment such as a CMM, and sometimes salespeople. Each unlock is a pipeline item on the Timeline tab.
• Salespeople are hired per market from the Markets tab, separately from plant workers on the People tab, since they work outside the plant. After a ramp-up they raise the order arrival rate in markets that run on relationships, and they cost salary plus commission.
• Order streams are generated per market: bursty arrivals, heavy-tailed order sizes, and for each order a part family and complexity, material, quantity, tolerance class, due date, inspection requirements and a market reference price.
Bidding policy (sensible defaults, editable)
Setting
Meaning
Price
Markup over estimated cost, or a target relative to the market's reference price
Load pricing
Raise prices automatically as the plant fills up
Lead time promise
Standard and expedited lead times to quote, checked against current capacity
Filters
Materials, processes, tolerance classes, minimum order value, maximum quantity
Exposure cap
Maximum share of capacity or backlog for this market
Win probability falls as price rises above the market reference and rises with shorter promised lead times and better reputation in that market. Higher prices win fewer, more profitable orders.
First market: online prototyping marketplace (modeled on platforms like Xometry). On such platforms the platform typically sets the payout and shops choose which jobs to accept, so here the policy is an acceptance policy: filters plus a minimum payout per machine hour. It needs no certs or salespeople, which makes it the natural starting market; its terms are rule-of-thumb guesses. Later markets (medical, aerospace, defense, oil and gas, contract manufacturing, Amazon products) are unlocked as the player chooses.
Penalties for late or bad work
• Late shipments: lower marketplace ratings and less access to jobs, possible payout reductions or cancellation, late-delivery penalties in direct contracts, expedited freight at the shop's expense, and lost repeat orders.
• Out-of-tolerance parts: customer rejection, return shipping, remakes at the shop's own cost, nonconformance reports and corrective-action paperwork (labor time), and chargebacks. In regulated markets a quality escape can trigger an audit or disqualification.
• Reputation per market tracks on-time delivery and quality, and feeds back into win probability and order flow.
• Penalty sizes are rule-of-thumb guesses.
Cell status: "Why isn't this running?"
Every cell shows a plain-language status, produced by its team from estimated state, which comes from sensor data, so the same status works in the game and in the real factory.
• Status contract: each team provides status(cell, estimated state) → state, reason, message, since, blocker. Reasons come from a shared vocabulary (waiting for an operator, waiting for material, output square full, setup in progress, down for repair, no power, waiting on a permit) plus team-specific detail text.
• Blockers are links: clicking jumps to the person, buffer, order or pipeline item causing the wait.
• Honest uncertainty: since status comes from the estimate, it can say "probably" and flags stale data ("no data for 10 minutes").
• Status appears in the detail view, as a floor label, and in the decision inbox when something has been blocked for long.
Cell lifecycle in the detail view
Stage
What the detail view shows
Choosing (before placing)
The team's options: cell type, add-ons and configuration, with live price, lead time range, footprint, utilities and requirements as choices change
Planned
The same options, still editable, plus validity checks for its spot and the plan's totals
In progress
An approximate progress bar, a checklist of the cell's build items and their status, and an ETA range
Commissioning
First-part and trial status, and the remaining checklist
Operational
Utilization, current job and queue, key distributions, recent scrap and downtime, costs, alerts, and actions (reconfigure, service, move, retire)
Down or in maintenance
Cause, repair status and ETA range
Build checklist and progress. Each cell type's team defines the cell's build checklist: the machine itself and each add-on, tooling and fixtures, utility work (power drop, air line), any permit, rigging, installation and commissioning. Each item is a pipeline item with its own lead-time distribution and status (ordered, shipped, delivered, installed). The detail view shows items as delivered or outstanding ("3 of 5 delivered"). The progress bar and ETA come from the controller's estimated state, so the ETA is a range (for example 10th to 90th percentile), shows when it was last updated, and moves as deliveries and vendor updates arrive. The same checklist appears as grouped bars on the Timeline tab.
Other selections use the same detail view: people (task, skills, ramp-up), buffers and squares (contents), trucks (contents, ETA), and pipeline items selected on the Timeline tab.
3D top-down rendering
• Low-fidelity 3D: extruded boxes, cylinders and polygons with flat colors, viewed from above by default with an optional tilt. Height carries meaning: racks are tall, squares are flat, people are short cylinders.
• Teams draw their cells. Each team provides a pure Rust function: visualize(cell type, configuration, estimated state) → scene nodes. Scene nodes reference the team's glTF assets and bind state to them (see Cell models and assets below), with generic primitives and labels for anything without an asset. Planned cells use the same function with a ghost style.
• The core draws everything else: the building shell (walls, columns, doors, docks, floor markings and aisles), people, handling units and containers, paths, and overlays (utilization, bottleneck, congestion, planning validity).
• The client renders only glTF assets and generic primitives with Three.js, using instanced meshes for people and containers. Teams never write TypeScript.
• Where scene functions run: on the server in wave 1, with scene nodes sent only when a cell's estimated state changes and animation driven by the hints in between. The same pure functions can later be compiled to WebAssembly and run in the browser if profiling favors it.
• Testing: scene functions are pure Rust and unit tested; the renderer stays a thin, untested layer per AGENTS.md.
Cell models and assets
Teams ship 3D models of their cells as glTF 2.0 (binary .glb), the open Khronos standard that Three.js loads natively. FBX is proprietary with weak web support, so it's used at most as an intermediate export.
• From CAD to glTF. Many CAD tools export glTF directly; otherwise STEP files are tessellated (for example with OpenCascade, FreeCAD or CadQuery) and cleaned up and decimated in Blender. Hidden internals are stripped and polygon counts kept low.
• Named nodes are the contract. Each asset exposes named nodes that the team's visualize function drives, such as door, spindle, table, status_light or bar_feeder. The team's Rust function stays the single owner of how state maps to visuals.
• State bindings. visualize returns, per cell: the asset id, transforms for named nodes (door angle, table position, spindle rate), material overrides (status light color), visibility toggles (show the bar feeder add-on), labels, and optionally a baked animation clip to play with a start time and speed. Snapping from state to state with no animation is fine.
• Configurations map to the asset through visibility toggles on optional nodes, or separate asset variants for very different configurations.
• Versioned forever. Assets are content-addressed and kept for every cell type, configuration and version ever shipped, like message schemas.
• CI checks every asset: required named nodes exist, units are meters, the origin and orientation follow convention, the bounding box matches the cell's declared footprint, and the polygon budget holds.
• Fallback. A cell with no asset is drawn by the core as a footprint box from catalog data, so wave 1 can ship with boxes and teams add models later.
• Delivery. The server sends asset ids and small binding diffs; the browser fetches each asset once by its content hash and caches it permanently.
• Confidentiality. Models of our own machines reveal design details, and models tied to defense work can count as export-controlled technical data. Assets are decimated to remove internals, served only to authenticated users, and ITAR-relevant assets are access-controlled like other controlled data.
Client architecture (following AGENTS.md)
• One GameClient class holds two things: a mirror of the server's estimated state, changed only by applying server updates; and UI-local state such as camera, selection, active tab, tool mode and planning drafts. Its methods (placeCell, setSpeed, skipAhead, select, executePlan) either send commands or change UI state.
• React subscribes through one version counter (useSyncExternalStore); useState is for cosmetic state only.
• Rendering stays outside React. A requestAnimationFrame loop turns the mirror into Three.js scene updates through pure, tested functions; React draws only panels, tabs and the detail view.
• The client never simulates. Logic lives in Rust; TypeScript holds the state class plus pure helpers that genuinely need the browser.
Protocol between client and server
• One WebSocket per open game. On connect the server sends a full snapshot of the estimated state; then updates (from_version, to_version, changed and removed entities, clock and speed) at up to 10–30 per second. On a version gap or reconnect, the client asks for a fresh snapshot and carries on.
• Simple, cheap updates. State uses simple ordered collections (`BTreeMap`, `Vec`) with monotonic entity version stamps (`u64`) and `Arc` boundaries, so finding changed entities since `from_version` is a fast, simple scan without exotic persistent data structures.
• Commands and queries. Each command carries an id and gets a result (ok or rejected with a reason). Read-only queries, such as validating a placement or fetching a distribution for the detail view, are request and response.
• Static data (catalogs, buildings, asset ids) is sent once and cached.
• One definition of every type. Wire schemas are defined once in `.proto` files in `fsim-protocols` and compiled to Rust (`prost`) and TypeScript (`ts-proto`), never written by hand in two places.
• Protobuf envelopes everywhere (both device-to-gateway and server-to-client, with JSON-encoded Protobuf available for human inspection/debugging).
• Testing: a headless Rust bot client plays scripted games over the same WebSocket protocol in CI, so the whole playable loop is tested without a browser.
Controls and settings. Pan, zoom and orbit with mouse or trackpad; click to select; keyboard shortcuts for speed, pause, skip ahead and tabs. Settings for units (metric or imperial), currency and auto-pause.
Report a problem. An in-game button files a report with a note, the game's seed, current checkpoint and log position. Because replay is deterministic, every report can be reproduced exactly.
Buildings and sites
One building model covers everything from a huge open warehouse to an oddly shaped urban building to a fabrication unit in a shipping container, so site choice is a real strategic lever.
The building model
Element
What it captures
Why it matters
Site
Parcel outline, yard and truck court, street access, neighbors
Truck turning room, trailer parking, outdoor storage, noise limits
Buildings
One or more per site, possibly multi-tenant
Shared docks and common areas in multi-tenant buildings
Levels
Polygon outlines with holes, elevation, mezzanines
Upper floors in urban buildings; mezzanines for light work and storage
Clear height
A height map per level, lower under beams and ducts
Rack height, crane and gantry feasibility, machine fit
Columns and walls
Column grid, fixed walls, rooms
Obstacles for layout and pathing; rooms for CMMs, offices, hazmat
Floor load
Slab capacity by zone, lower on upper floors
Heavy machines, presses, racks and forklift routes
Openings
Dock doors, grade-level roll-ups, personnel doors, freight elevators, stairs, ramps, each with dimensions
What can come in or out, and how big a machine can ever be installed
Utilities
Electrical service (amps, voltage, phase), panel locations, compressed air, water, drains, gas, HVAC
Which cells can run where; upgrades cost money and often months of utility lead time
Fire protection
Sprinkler type and coverage
Limits on rack storage height and hazardous materials
Rules
Zoning, permitted processes, operating hours, delivery windows
Comes from the site-selection and permitting work
Every cell on a team's menu declares its requirements (clear height, floor load, power, air, door size to get it in, permits), and the layout validator checks them against the building, so a press that can't fit through the door can't be planned there.
The range it covers
• Large open warehouse: big rectangle, high clear height, many dock doors, ample power. Few constraints; layout and flow dominate.
• Urban and odd buildings: irregular footprints, columns everywhere, low ceilings, multiple floors joined by a freight elevator, street loading with no dock, box trucks only, delivery time windows and noise limits. The freight elevator becomes a shared mover with a queue, capacity and dimensions, and often the bottleneck.
• Shipping container unit: a single small level of about 5.9 × 2.35 × 2.4 m (20 ft) or 12.0 × 2.35 × 2.7 m (40 ft high cube), with power from a generator or shore connection. It can be moved to a new site as freight, which allows strategies like small units placed near customers. Fit-out, generator fuel and transport have real costs.
Movement across levels. Each level has its own navigation grid; elevators, stairs and ramps connect them as portals with capacity and waiting time. People and forklift planning works per level, with the portal graph on top.
Geometry correctness. Arbitrary polygons with holes are where geometry code accumulates edge cases. Layout geometry uses integer millimeter coordinates and exact predicates in fsim-geom, so containment, clearance and overlap answers are correct for every shape.
Where buildings come from
1.  Wave 1: rectangles with a few doors, set by parameters.
2.  A polygon editor in the game for tracing a floor plan.
3.  Import from DXF drawings and IFC building models.
4.  Listings data, where a listing gives size, clear height, doors and power. Missing details are filled with a plausible generated footprint and marked as estimated.
Costs by building type. Rent per square foot and pass-through charges by market, tenant improvements and build-out, power upgrades, permits, and for containers the unit price, fit-out, generator and relocation freight. All are rule-of-thumb guesses for now.
Rendering. The core draws the building from this model: extruded walls and columns, doors, dock positions, clear height shown by shading, and a level selector for multi-floor buildings.
Support areas and the build palette
A job only makes sense if material can get in, be cut, made, checked, packed and shipped, and its waste hauled away, so the sim has a support area for each step and the palette lets the player place them like machines.
The build palette
• One button per team opens that team's cell menu (see Game client). Support areas are cells like any other: Quality's menu offers a manual inspection bench, a vision system or a CMM; Materials offers a receiving bench, racks and saws.
• Hovering a cell type or add-on shows a card: purchase price, monthly costs (lease, consumables, service), lead time range, footprint and clearances, utilities (power, air, water, climate control), staffing, a summary of its key conditional distribution, handling units its squares accept and produce, whether lenders will finance it, and requirements such as "needs a temperature-controlled room."
• Unavailable options stay visible but disabled, with the reason ("needs 480 V three-phase," "needs a hazmat permit").
• Areas can be kits. Placing "Receiving area" drops a default set of cells (bench, scale, label printer, staging squares) as planned cells the player can then edit.
• Menus come from team data, not core code, so options, add-ons and prices can be added and recalibrated without touching the sim.
The material path
Area
Owner
What happens
In → out
Receiving
Materials and logistics
Unload trucks, open boxes, check contents against the purchase order, check material certs and record heat or lot numbers for traceability, label, put away
Inbound pallets and boxes → labeled stock on storage racks
Stock storage
Materials and logistics
Hold bar, plate, resin, consumables and packaging; track remnants
Labeled stock → stock picked for jobs
Saw and stock prep
Materials and logistics
Cut bar and plate to blank size; a conversion station in handling-unit terms
Bar or plate → tray of blanks plus a remnant back to the rack
Production cells
Each production team
Machine, mold, heat treat, finish
Blanks or resin → parts
Inspection (optional)
Quality
First-article and sampled inspection; hold, release or reject
Parts → released parts, holds, scrap
Packing
Finishing and packing
Count, bag, box, label, palletize
Released parts plus packaging → sealed boxes or pallets
Shipping
Materials and logistics
Stage by carrier and cutoff time, weigh, label, load
Boxes and pallets → parcel, LTL or FBA shipments
Waste and recycling
Facilities and EHS
Collect, store and hand off chips, scrap, spent coolant, cardboard and trash to haulers
Waste → hauler pickups, with fees or scrap revenue
Waste is owned by Facilities and EHS rather than shipping because the rules are environmental: storage limits, containment, manifests and licensed haulers. Pickups still use dock time, so they are booked into the shipping dock schedule.
Options by area (prices and lead times are rule-of-thumb guesses)
Area
Option
Approx. price
Lead time
Notes
Receiving
Receiving bench kit (bench, scale, label printer)
$2k–$5k
Days
One person; minutes per box, more for cert checks
Receiving
Pallet jack
$0.5k–$1.5k
Days
Moves pallets from the dock; no license needed
Receiving
Forklift
$25k–$50k
2–12 weeks
Needs certified drivers and wide aisles; financeable
Storage
Cantilever rack (bar)
$2k–$6k per section
1–4 weeks
Long footprint; holds full-length bar and remnants
Storage
Pallet racking
$150–$400 per position
1–4 weeks
Resin gaylords, boxed stock; may need permit and anchoring
Storage
Shelving
$200–$800 per unit
Days
Small consumables, tooling, packaging
Saw
Manual horizontal bandsaw
$3k–$10k
1–3 weeks
Operator tends every cut
Saw
Automatic bar-feed bandsaw
$25k–$70k
4–10 weeks
Runs a bundle unattended; cut time scales with cross-section and material
Saw
Automatic carbide circular saw
$60k–$200k
8–16 weeks
Several times faster on solids, cleaner faces, pricier blades
Inspection
Manual bench (granite plate, height gauge, micrometers, pin gauges)
$5k–$20k
1–2 weeks
Minutes per feature; skill-dependent error
Inspection
Vision measuring system
$25k–$100k
4–8 weeks
Fast for 2D profiles
Inspection
Bridge CMM
$80k–$250k
8–16 weeks
Seconds per feature, programming free by assumption; needs a temperature-controlled room ($20k–$80k build-out)
Packing
Manual pack station
$2k–$6k
Days
Bench, tape, scale, label printer; minutes per box
Packing
Counting and bagging machine
$20k–$80k
6–12 weeks
Essential for small molded parts like forks
Packing
Case taper, stretch wrapper
$5k–$30k each
2–8 weeks
Pallet and carton throughput
Shipping
Staging lanes, shipping scale, label printer
$1k–$3k plus floor space
Days
Parcel cutoff times and LTL appointments drive staging
Shipping
Dock leveler
$5k–$15k
4–8 weeks
Needed for trailer loading without a liftgate
Waste
Self-dumping chip hoppers
$1k–$3k each
1–3 weeks
One per machine; full hoppers stop machines
Waste
Chip wringer or centrifuge
$15k–$50k
6–12 weeks
Recovers coolant and raises chip scrap value
Waste
Drum storage with spill containment
$0.3k–$1k per pallet
Days
Spent coolant and oils; accumulation time limits apply
Waste
Scrap and trash dumpsters
Monthly rental plus pickup fees
Days
Sorted aluminum chips and scrap earn revenue; mixed scrap earns less
Details that make it realistic
• Receiving creates traceability. Heat and lot numbers recorded here travel with every blank cut from that bar, which aerospace and medical customers require.
• Sawing creates remnants. Each cut leaves a remnant that goes back to the rack; reusing remnants saves material but costs search and handling time. Kerf loss is part of material cost.
• Packaging is inventory. Boxes, bags, dunnage and labels are ordered, received and stored like stock, and running out stops packing.
• Inspection plans vary by customer. First-article inspection, sampling plans or 100% inspection, set per job; a failed sample puts the lot on hold.
• CMMs need climate. Shop temperature swings put CMM results out of spec unless the room is controlled, which costs build-out and HVAC energy.
• Shipping has cutoffs. Parcel pickups happen at set times and LTL needs appointments; missing a cutoff adds a day.
• Waste has limits. Spent coolant and oils can only be stored for limited periods before a licensed hauler must take them, with manifests and per-drum fees. Full chip hoppers stop machines, which is a classic unattended-shift failure.
Wave 1 includes a receiving bench, cantilever rack, manual bandsaw, manual inspection bench, manual pack station, shipping staging, and chip hoppers with a scrap dumpster. The automated options come in later waves.
Build order and open questions
Build the deterministic core and one end-to-end slice first, then widen.
1.  Core and one slice. Clock, event log, world state, ledger, protocols; one CNC machine, one operator, bar stock in, parts out by parcel carrier; simplest fidelity levels; golden-replay test.
2.  Game server and viewer. Sign-in required from the start, two-factor authentication soon after. Actor-per-game server, WebSocket diffs, 3D top-down viewer with play, pause, speed, fork and rewind; planning mode and cell placement with delivery and forklift moves.
3.  Floor breadth. People with time standards, inventory and material tracking, docks and trucks, pull rules, incidents.
4.  Molding and the toolroom. Mold build, heat treat options, presses with process windows, packing.
5.  Solvers and search. Capacity model with shadow prices, layout solver, coarse sim calibrated from the detailed sim, MCTS over the formal game, "run MCTS from here."
6.  Baseline validation. Prediction evaluator sanity checks on sim plants (zero-mean, known-variance errors); a conventional-shop scenario reproducing industry benchmarks; then R&D lever sweeps and group play.
7.  Twin. Event ingestion from our controllers, snapshots, conditional model scoring, model registry, decision log, ops dashboards.
First playable release checklist
[x] Invite links from the root admin, account creation, sign-in, sign-out and admin-generated password reset; each user's games listed on the setup screen
[x] Game setup screen: new game, load checkpoint, live factory greyed out
[x] Plants tab with leasing, keys, utilities and build-out lead times; a new game starts with no plants
[x] Markets tab with the online prototyping marketplace, an acceptance policy with sensible defaults, stochastic orders, and penalties for late or out-of-tolerance work
[x] Floor, Timeline, Ledger and People tabs, the detail view and planning mode
[x] "Why isn't this running?" status for every cell
[x] Decision inbox and alerts, tied to auto-pause
[x] Sensible defaults: dispatch rules, task priorities, shifts, reorder points
[x] Small starting catalogs: about 3–5 options per team
[x] Skippable tutorial (lease a plant, one CNC cell, one packing station, one employee, marketplace on with a middle price policy), then sandbox play
[x] Autosave, named checkpoints, rewind, and undo in planning mode
[x] No ending except bankruptcy: the game continues indefinitely and ends only when the company runs out of cash and credit
[x] Client architecture and WebSocket protocol, with reconnect and a headless bot client in CI
[x] Starter content pack: 3–5 cell options per team with guessed numbers, a handful of building listings, prototyping marketplace order parameters, one starting scenario
[x] Hosting on a single Linux or Mac machine: one server binary, SQLite for users, game data as files in one folder
[x] Basic controls, keyboard shortcuts, and unit and currency settings
[x] "Report a problem" button with seed, checkpoint and log position