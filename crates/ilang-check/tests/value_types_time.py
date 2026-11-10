#!/usr/bin/env python3
"""T2 checklist 9.11: one warm-up, 11 release std checks per compiler."""
import argparse
import json
import statistics
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--baseline", required=True)
parser.add_argument("--candidate", required=True)
parser.add_argument("--project", required=True)
args = parser.parse_args()
result = {}
for name, compiler in [("baseline", args.baseline), ("candidate", args.candidate)]:
    samples = []
    for run in range(12):
        start = time.perf_counter()
        subprocess.run([compiler, "check", "--project", args.project], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        elapsed = 1000 * (time.perf_counter() - start)
        if run:
            samples.append(elapsed)
    result[name] = {"milliseconds": samples, "median": statistics.median(samples)}
result["regression_percent"] = 100 * (result["candidate"]["median"] / result["baseline"]["median"] - 1)
print(json.dumps(result, indent=2))
if result["regression_percent"] > 10:
    raise SystemExit("std median exceeds the accepted +10% investigation threshold")
