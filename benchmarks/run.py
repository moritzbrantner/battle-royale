#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
COLLECTOR_NAME = "battle-royale.performance-receipt"
COLLECTOR_VERSION = "1"
REPOSITORY_URI = "https://github.com/moritzbrantner/battle-royale"


def run(*args: str) -> str:
    return subprocess.run(
        args,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def canonical_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def git_dirty() -> bool:
    return bool(run("git", "status", "--porcelain=v1", "--untracked-files=all"))


def cpu_model() -> str:
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.lower().startswith("model name"):
                return line.partition(":")[2].strip()
    return platform.processor() or "unknown"


def nearest_rank(values: list[float], percentile: float) -> float:
    ordered = sorted(values)
    rank = max(1, math.ceil(percentile * len(ordered)))
    return ordered[rank - 1]


def measurement(
    name: str,
    value: int | float,
    unit: str,
    measurement_type: str,
    description: str,
) -> dict[str, Any]:
    return {
        "name": name,
        "value": value,
        "unit": unit,
        "measurement_type": measurement_type,
        "description": description,
    }


def environment() -> dict[str, Any]:
    details = {
        "platform": {
            "os": platform.system().lower(),
            "arch": platform.machine().lower(),
            "cpu": cpu_model(),
        },
        "toolchain": {
            "rustc": run("rustc", "--version"),
            "python": platform.python_version(),
        },
        "collector": {
            "name": COLLECTOR_NAME,
            "version": COLLECTOR_VERSION,
        },
    }
    return {
        "fingerprint": canonical_hash(details),
        **details,
    }


def source() -> dict[str, Any]:
    return {
        "repository": REPOSITORY_URI,
        "revision": run("git", "rev-parse", "HEAD"),
        "dirty": git_dirty(),
    }


def scenario(
    scenario_id: str,
    description: str,
    workload_id: str,
    parameters: dict[str, Any],
) -> dict[str, Any]:
    return {
        "id": scenario_id,
        "description": description,
        "workload": {
            "id": workload_id,
            "hash": canonical_hash(parameters),
            "parameters": parameters,
        },
    }


def evidence(
    scenario_value: dict[str, Any],
    source_value: dict[str, Any],
    environment_value: dict[str, Any],
    useful_work: list[dict[str, Any]],
    induced_work: list[dict[str, Any]],
    outcomes: list[dict[str, Any]],
    samples: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "scenario": scenario_value,
        "source": source_value,
        "environment": environment_value,
        "measurements": {
            "useful_work": useful_work,
            "induced_work": induced_work,
            "outcomes": outcomes,
        },
        "extensions": {
            "battle-royale.samples": samples,
        },
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Collect full-capacity Battle Royale performance evidence."
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "performance-results",
    )
    args = parser.parse_args()
    output_dir = args.output_dir
    if not output_dir.is_absolute():
        output_dir = ROOT / output_dir

    dirty_before_collection = git_dirty()

    probe_result = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--release",
            "--locked",
            "-p",
            "battle-royale-game-server",
            "--example",
            "performance_probe",
        ],
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    probe = json.loads(probe_result.stdout)

    required = {
        "players",
        "match_id",
        "pre_measure_ticks",
        "ticks_per_batch",
        "warmup_batches",
        "measured_batches",
        "tick_batch_ns",
        "canonical_bytes",
        "player_projection_bytes_total",
        "player_projection_bytes_max",
        "player_projection_bytes_min",
    }
    missing = sorted(required.difference(probe))
    if missing:
        raise RuntimeError(f"performance probe omitted fields: {', '.join(missing)}")

    tick_samples = [
        value / probe["ticks_per_batch"] for value in probe["tick_batch_ns"]
    ]
    if len(tick_samples) != probe["measured_batches"]:
        raise RuntimeError("performance probe sample count does not match measured_batches")

    source_value = source()
    source_value["dirty"] = dirty_before_collection
    environment_value = environment()

    shared_parameters = {
        "players": probe["players"],
        "match_id": probe["match_id"],
        "pre_measure_ticks": probe["pre_measure_ticks"],
        "movement_pattern": "player-id-modulo-3-v1",
    }

    tick_parameters = {
        **shared_parameters,
        "ticks_per_batch": probe["ticks_per_batch"],
        "warmup_batches": probe["warmup_batches"],
        "measured_batches": probe["measured_batches"],
    }
    tick_document = evidence(
        scenario(
            "battle-royale/full-capacity-tick",
            "Release-mode authoritative combat tick cost for a deterministic 100-player movement workload with active storm evaluation.",
            "full-capacity-movement-v1",
            tick_parameters,
        ),
        source_value,
        environment_value,
        [
            measurement(
                "battle-royale.players",
                probe["players"],
                "count",
                "gauge",
                "Authoritative players retained throughout each measured tick batch.",
            ),
            measurement(
                "battle-royale.ticks-per-batch",
                probe["ticks_per_batch"],
                "count",
                "counter",
                "Authoritative ticks advanced by one measured batch.",
            ),
        ],
        [],
        [
            measurement(
                "time.tick-median",
                statistics.median(tick_samples),
                "ns",
                "duration",
                "Median elapsed nanoseconds per authoritative tick across measured batches.",
            ),
            measurement(
                "time.tick-p95",
                nearest_rank(tick_samples, 0.95),
                "ns",
                "duration",
                "Nearest-rank p95 elapsed nanoseconds per authoritative tick across measured batches.",
            ),
        ],
        {
            "tick_batch_ns": probe["tick_batch_ns"],
            "ticks_per_batch": probe["ticks_per_batch"],
        },
    )

    projection_parameters = {
        **shared_parameters,
        "recipients": probe["players"],
    }
    projection_document = evidence(
        scenario(
            "battle-royale/full-capacity-projection-bytes",
            "Encoded canonical and player-scoped snapshot bytes for the deterministic full-capacity workload.",
            "full-capacity-projection-v1",
            projection_parameters,
        ),
        source_value,
        environment_value,
        [
            measurement(
                "battle-royale.players",
                probe["players"],
                "count",
                "gauge",
                "Players present in the authoritative full-capacity match.",
            ),
            measurement(
                "battle-royale.player-projections",
                probe["players"],
                "count",
                "counter",
                "Player-scoped snapshots encoded once for every recipient.",
            ),
        ],
        [],
        [
            measurement(
                "protocol.canonical-snapshot-bytes",
                probe["canonical_bytes"],
                "byte",
                "size",
                "Encoded canonical recovery/replay snapshot size.",
            ),
            measurement(
                "protocol.player-projection-bytes-total",
                probe["player_projection_bytes_total"],
                "byte",
                "size",
                "Total encoded bytes across one player-scoped projection per recipient.",
            ),
            measurement(
                "protocol.player-projection-bytes-max",
                probe["player_projection_bytes_max"],
                "byte",
                "size",
                "Largest encoded player-scoped projection in the workload.",
            ),
            measurement(
                "protocol.player-projection-bytes-min",
                probe["player_projection_bytes_min"],
                "byte",
                "size",
                "Smallest encoded player-scoped projection in the workload.",
            ),
        ],
        {
            "canonical_bytes": probe["canonical_bytes"],
            "player_projection_bytes_total": probe["player_projection_bytes_total"],
            "player_projection_bytes_max": probe["player_projection_bytes_max"],
            "player_projection_bytes_min": probe["player_projection_bytes_min"],
        },
    )

    write_json(output_dir / "tick-cost.json", tick_document)
    write_json(output_dir / "projection-bytes.json", projection_document)

    summary = [
        "# Battle Royale performance evidence",
        "",
        f"- Source: `{source_value['revision']}` (dirty: `{str(source_value['dirty']).lower()}`)",
        f"- Players: {probe['players']}",
        f"- Tick median: {statistics.median(tick_samples):.1f} ns/tick",
        f"- Tick p95: {nearest_rank(tick_samples, 0.95):.1f} ns/tick",
        f"- Canonical snapshot: {probe['canonical_bytes']} bytes",
        f"- Player projections total: {probe['player_projection_bytes_total']} bytes",
        f"- Largest player projection: {probe['player_projection_bytes_max']} bytes",
        f"- Smallest player projection: {probe['player_projection_bytes_min']} bytes",
        "",
        "Wall-clock timing is advisory performance evidence, not a correctness threshold. "
        "Projection byte counts are recorded independently from the ordinary deterministic test gate.",
        "",
    ]
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "summary.md").write_text("\n".join(summary), encoding="utf-8")

    print("\n".join(summary))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"performance evidence collection failed: {error}", file=sys.stderr)
        raise SystemExit(1)
