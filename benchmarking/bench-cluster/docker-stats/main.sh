#!/bin/bash

rm -f /shared-volume/docker-stats.log

while true; do
    timestamp=$(date +%s)
    stats_json=$(docker stats --no-stream --format '{{json .}}' | jq -s -c)

    # Get container names array
    container_names=$(echo $stats_json | jq -r '.[].Name')

    # Filter out "docker-stats" and "scrapi"
    filtered_stats_json=$(echo $stats_json | jq -c 'map(select(.Name != "docker-stats" and .Name != "scrapi"))')
    
    # Break if no containers are running
    if [ "$(echo "$filtered_stats_json" | jq 'length')" -eq 0 ]; then
        break
    fi

    echo "{\"timestamp\": $timestamp, \"stats\": $stats_json}" >> /shared-volume/docker-stats.log
    echo "{\"timestamp\": $timestamp, \"stats\": $stats_json}"
done

# Shutdown "scrapi"
curl http://scrapi:8000/shutdown > /dev/null 2>&1
exit 0