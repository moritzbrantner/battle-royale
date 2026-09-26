# Battle Royale roadmap

## Foundation — authoritative match kernel

- [x] Establish Rust workspace, exact dependency pins and fail-closed validation.
- [x] Define deterministic match identity, player capacity and ordered state.
- [x] Delegate movement/collision integration to `physics-engine`.
- [x] Define Waiting -> Drop -> Combat -> Finished lifecycle.
- [x] Add deterministic safe-zone shrink and storm elimination.
- [x] Keep canonical recovery snapshots separate from player-scoped projections.
- [x] Adapt the match kernel to `game-server::GameSimulation`.
- [x] Prove replay/recovery and reconnect continuity through `game-server`.
- [x] Prove multiple matches can be hosted without sharing gameplay authority.

## Runnable authoritative host

- [x] Add a Battle Royale host binary around `game-server::MatchHost`.
- [x] Expose health/readiness/status through the reusable host status contract.
- [x] Add graceful hosted recovery for a complete configured match set.
- [x] Keep operational status read-only and separate from gameplay mutation.
- [ ] Add real WebTransport acceptance for two clients in one match and two isolated matches.

## Combat model

- [ ] Define server-owned weapon/fire/reload commands and deterministic cooldowns.
- [ ] Add authoritative hit validation without trusting client hit results.
- [ ] Keep projectile/character collision in `physics-engine`; Battle Royale owns damage rules.
- [ ] Add knockdown/revive only after solo elimination semantics remain explicit.
- [ ] Make all combat state replay-complete before adding cosmetic prediction.

## Loot and inventory

- [ ] Define deterministic loot spawn tables from match seed + content revision.
- [ ] Add bounded inventory/equipment authority and pickup/drop commands.
- [ ] Keep item content/provenance in reusable asset/content foundations.
- [ ] Add transaction-safe pickup conflict handling and replay evidence.

## Drop and traversal

- [ ] Replace placeholder spawn slots with an authoritative transport/drop path.
- [ ] Model jump, glide and landing as server-owned state transitions.
- [ ] Add terrain/static collision content through shared physics/content foundations.
- [ ] Add vehicles only after player traversal correctness and recovery are stable.

## Networking and scale

- [x] Bound canonical and player-scoped snapshot sizes at the full 100-player capacity.
- [ ] Add spatial indexing only when deterministic evidence shows the bounded O(n) projection is insufficient.
- [ ] Add snapshot delta/compression only behind versioned protocol evidence.
- [ ] Keep one match single-writer; scale by hosting more matches, not by splitting one physics step across machines.
- [ ] If a future mega-map requires intra-match partitioning, design explicit authority transfer rather than shared mutable simulation.

## Matchmaking and services

- [ ] Integrate match creation/placement with existing multiplayer setup/service foundations.
- [ ] Keep account, party, ranking and entitlement state outside the match hot loop.
- [ ] Fence delayed external side effects with match/session identity.
- [ ] Add durable match result publication as an idempotent post-match command.

## Client foundations

- [ ] Reuse pinned `3d-lab` rendering rather than creating a renderer here.
- [x] Reuse the shared `input-bindings` action model for Battle Royale client actions.
- [x] Add deterministic mobile thumbstick adaptation onto the existing authoritative movement command.
- [x] Keep look/menu mobile interactions client-local until game rules require an authoritative command.
- [ ] Compose the shared mobile overlay editor into the first rendered Battle Royale client once that client surface exists.
- [ ] Reuse `settings` and `asset-tooling`.
- [ ] Add interpolation over authoritative player-scoped snapshots.
- [ ] Add optional shared-physics prediction/reconciliation only after acknowledgement contracts are explicit.
- [ ] Keep browser/Pages demos non-authoritative.

## Evidence

- [x] Add deterministic 100-player storm/movement workload fixtures.
- [x] Add performance receipts for tick cost and projection bytes separately from correctness.
- [ ] Add packet impairment/reconnect acceptance using the existing `game-server` transport harness patterns.
- [ ] Add long-run replay equivalence and recovery corruption tests.
