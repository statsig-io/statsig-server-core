#!/usr/bin/env python3

import subprocess
import json
import re
import time
import sys
import os


def run_bench(bin_name: str, display_name: str) -> float:
    print(f"Running {display_name} benchmark...")

    rss_value = None
    attempts = 0

    while rss_value is None and attempts < 3:
        try:
            env = os.environ.copy()
            env["MIMALLOC_SHOW_STATS"] = "1"
            # Run the cargo command with mimalloc stats enabled
            result = subprocess.run(
                ["cargo", "run", "--bin", bin_name],
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,  # Line buffered
            )

            print(result.stderr)
            print(result.stdout)

            # Search for RSS value in stderr (mimalloc stats are printed to stderr)
            rss_match = re.search(r"rss: ([0-9.]+) MiB", result.stderr)

            if rss_match:
                rss_value = float(rss_match.group(1))
            else:
                attempts += 1
                print(f"No RSS value found, attempt {attempts} of 3...")
                if attempts == 3:
                    print(
                        f"Error: Failed to get RSS value after 3 attempts for {display_name}"
                    )
                    sys.exit(1)
                time.sleep(1)  # Small delay between attempts

        except subprocess.CalledProcessError as e:
            print(f"Error running benchmark: {e}")
            sys.exit(1)

    print(f"mimalloc stats for {display_name}")
    print(f"Resident Set Size (RSS) for {display_name}: {rss_value:.2f} MB")

    return rss_value


if __name__ == "__main__":
    # Initialize results list
    results = []

    # Run gate-mem benchmark
    gate_mem_result = run_bench("gate-mem", "gate mem")
    results.append({"name": "gate mem", "value": gate_mem_result, "unit": "MB"})

    # Run spec-sync-mem benchmark
    spec_sync_result = run_bench("spec-sync-mem", "spec sync mem")
    results.append({"name": "spec sync mem", "value": spec_sync_result, "unit": "MB"})

    # Write results to JSON file
    with open("bench_results.json", "w") as f:
        json.dump(results, f, indent=2)

    print("Wrote benchmark results to bench_results.json")
