#! /bin/bash

set -e

echo "-- Starting Smoke Test --"

EXTERNAL_IP=$1

if [ -z "$EXTERNAL_IP" ]; then
  echo "Missing required argument: $0 <external_ip>"
  exit 1
fi

echo "External IP: $EXTERNAL_IP"

function ensure_all_running() {
    CMD="curl -s http://$EXTERNAL_IP/stats"
    RESPONSE=$($CMD)

    if [ -z "$RESPONSE" ]; then
        return 1
    fi

    NAMES=""
    if [ -n "$RESPONSE" ]; then
        if echo "$RESPONSE" | jq -e . >/dev/null 2>&1; then
            NAMES=$(echo "$RESPONSE" | jq -r '.dockerStats.stats[].Name' 2>/dev/null)
        fi
    fi

    if [ -z "$NAMES" ]; then
        return 1
    fi

    EXPECTED_NAMES=(
        "sr-scrapi"
        "sr-docker-stats"
        "sr-dashboard"
        "sr-node-legacy"
        "sr-node-core"
        "sr-java-core"
        "sr-java-legacy"
        "sr-python-legacy"
        "sr-python-core"
    )

    # check that all services are running, regarldess of order
    for NAME in "${EXPECTED_NAMES[@]}"; do
        if ! echo "$NAMES" | grep -q "$NAME"; then
            return 1
        fi
    done

    return 0
}

echo "-- Checking Services are Stable --"

for i in {1..5}; do
    if ensure_all_running; then
        break
    fi
    echo "Waiting for all Services to Start (attempt $i/5)"
    sleep 1
done

for i in {1..5}; do
    if ! ensure_all_running; then
        echo "❌ Some Services Have Stopped"
        exit 1
    fi
    echo "Ensuring all Services are Running (attempt $i/5)"
    sleep 1
done

echo "✅ All Services are Running"
exit 0