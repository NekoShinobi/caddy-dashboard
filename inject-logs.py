#!/usr/bin/env python3
"""
Stream access-gen.log entries one line at a time into access.log.

Usage:
  python3 inject-logs.py            # one line, then exit
  python3 inject-logs.py --loop     # one line every 1000ms, cycling forever
  python3 inject-logs.py --loop --interval 500   # every 500ms
"""

import argparse
import itertools
import json
import sys
import time
from pathlib import Path

SRC = Path("access-gen.log")
DST = Path("access.log")


def load_entries() -> list[dict]:
    lines = [l for l in SRC.read_text().splitlines() if l.strip()]
    entries = []
    for i, line in enumerate(lines, 1):
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError as e:
            print(f"skipping line {i}: {e}", file=sys.stderr)
    return entries


def append_entry(entry: dict) -> None:
    e = dict(entry)
    e["ts"] = time.time()
    with DST.open("a") as f:
        f.write(json.dumps(e, separators=(",", ":")) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--loop", action="store_true", help="cycle through entries indefinitely")
    parser.add_argument("--interval", type=int, default=1000, metavar="MS",
                        help="ms between lines (default: 1000)")
    args = parser.parse_args()

    entries = load_entries()
    if not entries:
        print("no valid entries in access-gen.log")
        return

    if not args.loop:
        append_entry(entries[0])
        print(f"appended 1 entry to {DST}")
        return

    sleep = args.interval / 1000.0
    print(f"streaming one line every {args.interval}ms — Ctrl+C to stop")
    try:
        for entry in itertools.cycle(entries):
            append_entry(entry)
            time.sleep(sleep)
    except KeyboardInterrupt:
        print("\nstopped")


if __name__ == "__main__":
    main()
