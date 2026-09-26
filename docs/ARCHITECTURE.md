# Architecture

## Scope

A Battle Royale match is a bounded, single-writer authoritative simulation. Horizontal scale comes from hosting more independent matches through `game-server::MatchHost`; the first design does not distribute one match's physics step across machines.

```text
client input
     |
     v
battle-royale-client
     |
     v
authoritative commands
     |
     v
matchmaking / placement
        |
        v
 game-server MatchHost
   |          |
 match A    match B
   |          |
 BattleRoyaleGameServerAdapter
        |
        v
 battle-royale-core
        |
        v
  physics-engine
```

## Authority boundaries

### battle-royale-client

Owns consumer-specific client adaptation, not input-device semantics or match truth:

- declares Battle Royale semantic actions through the pinned `input-bindings` model;
- converts normalized mobile thumbstick samples into the existing discrete movement intent;
- suppresses duplicate movement commands when high-frequency pointer samples quantize to the same intent;
- treats look/camera and menu interaction as client-local until a game rule requires a corresponding authoritative command.

The reusable `input-bindings` repository owns the mobile overlay editor and binding semantics. Battle Royale must not fork that UI or introduce a second touch resolver.

### battle-royale-core

Owns only game-specific match truth:

- phase progression and lobby closure;
- accepted movement intent;
- deterministic spawn slots;
- safe-zone/storm schedule;
- health, elimination and winner state;
- which match state is visible to a player.

It does not own transport connection epochs, reconnect capabilities, TLS, host draining, replay storage, matchmaking or rendering.

### game-server

Owns reusable runtime authority:

- player/session identity and connection epochs;
- command sequence admission before game-specific decode;
- authoritative tick scheduling;
- player-scoped snapshot delivery;
- replay and graceful recovery;
- multi-match process hosting and operational lifecycle.

The adapter converts bytes and delegates; it does not create a second scheduler or session registry.

### physics-engine

Owns movement integration and collision semantics. Battle Royale supplies player movement intent and reads authoritative body state. Physics algorithms must not be copied into the game crate.

## Deterministic lifecycle

`Waiting` admits players until the deterministic lobby countdown expires. The match then closes admission and enters `Drop`, followed by `Combat`. During combat the safe-zone radius is computed from integer constants and authoritative tick only. No system clock or random number source participates.

The safe-zone center is deterministically derived from `BattleMatchId`, making a fresh replay simulation reconstruct the same world without hidden ambient seed state.

`Finished` is terminal. Runtime ticks may continue for transport/replay bookkeeping, but gameplay movement and damage no longer mutate.

## Disconnect semantics

Temporary network disconnect is a `game-server` concern and does not immediately change Battle Royale truth. The game simulation sees `remove_player` only when the reusable session runtime retires the slot. Before the match starts that removes the lobby participant; after the match starts it is a terminal `SessionExpired` elimination.

This prevents brief transport loss from becoming a second, accidental game-rule authority.

## Snapshot boundary

Canonical snapshots contain every authoritative player plus private sequencing/movement state and are used for replay/recovery hashing.

Player-scoped snapshots contain only the addressed player's allowed projection. The current first slice uses a bounded distance filter and always includes the addressed player. Future stealth, inventory or spectator policy belongs in this projection boundary, never in transport code.

## Capacity

The initial 100-player cap is a correctness bound, not a throughput claim. Raising it requires deterministic workload evidence for physics, tick work and per-player projection size. The current projection is intentionally simple O(n) over at most 100 players; a spatial index should be introduced from measured need rather than pre-emptively duplicating another repository's data structure.

## Persistence and external services

Durable match results, accounts, parties, rankings and matchmaking use command/query boundaries outside the hot loop. External service retries must be idempotent and fenced with match/session identity. Event sourcing is not required: `game-server` replay/recovery already provides deterministic runtime evidence, while durable business state can use simpler persistence contracts.
