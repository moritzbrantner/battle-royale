#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate Battle Royale evidence against a pinned Performance Evidence checkout."
    )
    parser.add_argument("contract_root", type=Path)
    parser.add_argument("evidence", nargs="+", type=Path)
    args = parser.parse_args()

    scripts = args.contract_root / "scripts"
    schema_path = args.contract_root / "schema" / "performance-evidence.schema.json"
    if not scripts.is_dir() or not schema_path.is_file():
        raise RuntimeError("contract_root is not a Performance Evidence checkout")

    sys.path.insert(0, str(scripts))
    from jsonschema import Draft202012Validator, FormatChecker  # type: ignore
    from validate_schema import validation_errors  # type: ignore

    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    validator = Draft202012Validator(schema, format_checker=FormatChecker())

    failed = False
    for path in args.evidence:
        document = json.loads(path.read_text(encoding="utf-8"))
        errors = validation_errors(validator, document)
        if errors:
            failed = True
            print(f"{path} failed Performance Evidence validation:", file=sys.stderr)
            for error in errors:
                print(f"- {error}", file=sys.stderr)
        else:
            print(f"{path} conforms to Performance Evidence 1.0.0")

    return 1 if failed else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, json.JSONDecodeError) as error:
        print(f"performance evidence validation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
