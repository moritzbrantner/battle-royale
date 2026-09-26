# Agent guidance

## Authority

- `battle-royale-core` owns deterministic match rules: lifecycle phases, storm/safe-zone state, elimination semantics, spawn allocation, authoritative player intent and player-visible interest policy.
- `battle-royale-client` owns Battle Royale-specific client adaptation only: semantic action declarations and normalized client input -> authoritative command mapping. It must not become a second source of match truth or fork reusable device/input semantics.
- `physics-engine` owns collision and movement integration. Do not reimplement a second physics engine here.
- `game-server` owns session identity, sequencing, tick scheduling, reconnects, replay/recovery, WebTransport and multi-match process hosting. Keep Battle Royale adapters thin.
- Matchmaking, accounts, parties, rankings and fleet placement remain outside match simulation authority. Integrate existing services at explicit boundaries instead of importing their state into the hot loop.

## Match invariants

- A match has exactly one authoritative simulation writer.
- Player commands are monotonically sequenced and stale or duplicate sequences fail closed.
- Temporary transport disconnects do not eliminate a player; only session retirement invokes game-level removal.
- Once the lobby closes, new player admission fails closed.
- The storm schedule is integer-only and deterministic from match identity plus authoritative tick.
- Elimination is terminal for the match. Finished matches cannot resume gameplay mutation.
- Canonical snapshots contain complete replay/recovery truth. Player-scoped snapshots are projections and must never become recovery authority.

## Determinism and performance

- Stable IDs and ordered collections are required in authoritative state.
- No wall-clock time, ambient randomness, unordered iteration or frontend state may influence match truth.
- Capacity changes require deterministic workload evidence; correctness tests must not depend on timing thresholds.
- Keep durable persistence and external service calls outside the simulation tick.

## Dependencies

- External reusable foundations are pinned to exact revisions.
- Update pins intentionally with compatibility evidence.
- `input-bindings` owns device/binding semantics and the reusable mobile overlay editor; Battle Royale may only supply consumer actions and adapters.
- Do not fork `game-server`, `physics-engine`, input, settings, rendering or asset foundations into this repository.

## Validation

Run formatting, Clippy, tests and build for the complete workspace with the committed lockfile. Do not weaken failures to make a slice green.
