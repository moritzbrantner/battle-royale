# battle-royale

Server-authoritative Battle Royale foundation built on the same reusable multiplayer and physics boundaries as `mmorpg`.

This first implementation slice is deliberately about authority, determinism and recovery rather than presentation. It provides a deterministic 100-player match simulation, lobby/drop/combat/finished phases, safe-zone shrink, storm elimination, player-scoped projections and a thin `game-server` adapter. Rendering, weapons, loot, matchmaking and deployment orchestration remain separate follow-up boundaries.

## Authority map

| Concern | Authority |
| --- | --- |
| Match phases, storm, spawn slots, elimination and visibility policy | `battle-royale-core` |
| Collision and movement integration | pinned `physics-engine` |
| Battle Royale wire encoding | `battle-royale-protocol` |
| Session ticks, command sequencing, reconnects, replay/recovery and process hosting | pinned `game-server` |
| Matchmaking/accounts/parties/rankings | external services, not match simulation |
| Rendering/input/settings/assets | existing reusable client foundations in later slices |

The repository composes those foundations; it should not grow substitutes for them.

## Implemented foundation

- deterministic integer-only match state with ordered player identity;
- bounded 100-player admission while the lobby is open;
- a deterministic lobby countdown and explicit Drop -> Combat -> Finished lifecycle;
- movement delegated to the shared `physics-engine`;
- exact monotonic game-command sequencing in core in addition to runtime fencing;
- deterministic safe-zone center and integer shrink schedule derived from match identity;
- terminal storm/session-expiry elimination and deterministic winner selection;
- canonical complete snapshots for replay/recovery;
- player-scoped snapshots with authoritative interest filtering;
- a thin `game-server::GameSimulation` adapter;
- deterministic mapping from Battle Royale match IDs to route-safe `game-server::MatchId` values;
- bounded multi-match host construction without duplicating host/session policy;
- recovery tests that restore match state and reconnect identity through `game-server`;
- exact dependency pins, committed lockfile and fail-closed workspace validation.

## Pinned foundations

- `game-server`: `769de47005cc37891011fc76ae183c18b7c5e0ae`
- `physics-engine`: `c796ea382bdcb0276b9309e8a3cca34c8c28313b`
- reusable validation workflow: `728fffa13c451766d08f06e6c7d7950a4de57b3d`

The pins intentionally match the already-proven MMORPG integration surface for the first slice. Upgrades should be explicit compatibility work, not floating dependency drift.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --all-features --locked
```

See [the architecture](docs/ARCHITECTURE.md) and [roadmap](ROADMAP.md).
