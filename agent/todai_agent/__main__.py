"""Entry point: `python -m todai_agent` or `todai-agent` (after pip install)."""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from . import __version__
from .agent import run_once
from .config import load, resolve_root


def main() -> int:
    parser = argparse.ArgumentParser(prog="todai-agent", description="todai AI loop")
    parser.add_argument("--path", type=Path, help="todai store root (default: $TODAI_HOME or ~/.todai)")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print notifications instead of sending; do not mark notified",
    )
    parser.add_argument(
        "--binary",
        default="todai",
        help="path to the todai binary (default: 'todai' on PATH)",
    )
    parser.add_argument("--verbose", "-v", action="store_true")
    parser.add_argument("--version", action="version", version=f"todai-agent {__version__}")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )

    root = resolve_root(args.path)
    config = load(root)
    summary = run_once(root, config, dry_run=args.dry_run, binary=args.binary)

    print(
        f"agent run: scanned={summary.scanned} fired={summary.fired} "
        f"smart={summary.smart} dumb={summary.dumb} errors={len(summary.errors)}"
    )
    for err in summary.errors:
        print(f"  ! {err}", file=sys.stderr)
    return 0 if not summary.errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
