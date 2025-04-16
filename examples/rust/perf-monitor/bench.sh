#! /bin/bash

echo "Running program..."
rss_value=""
attempts=0
while [ -z "$rss_value" ] && [ $attempts -lt 3 ]; do
    rss_value=$(MIMALLOC_SHOW_STATS=1 cargo run 2>&1 | grep "rss:")
    if [ -z "$rss_value" ]; then
        echo "No RSS value found, attempt $((attempts + 1)) of 3..."
        attempts=$((attempts + 1))
        if [ $attempts -eq 3 ]; then
            echo "Error: Failed to get RSS value after 3 attempts"
            exit 1
        fi
        sleep 1  # Add a small delay between attempts
    fi
done

echo "mimalloc stats"
echo "$rss_value"

# eg: "process: user: 3.567 s, system: 0.140 s, faults: 955, rss: 123.2 MiB, commit: 1.0 GiB"
rss_bytes=$(echo "$rss_value" | grep -o 'rss: [0-9.]\+ MiB' | grep -o '[0-9.]\+' | awk '{ printf "%.0f", $1 * 1024 * 1024 }')
rss_mb=$(echo "scale=2; $rss_bytes / (1024*1024)" | bc)

formatted_rss_bytes=$(printf "%'d" $rss_bytes)
formatted_rss_mb=$(printf "%.2f" $rss_mb)

echo "Resident Set Size (RSS): ${formatted_rss_mb} MB"

# Write the RSS value to a JSON file
echo "[{ \"name\": \"My Bench\", \"value\": $formatted_rss_mb, \"unit\": \"MB\" }]" > bench_results.json
echo "Wrote RSS value to bench_results.json"
