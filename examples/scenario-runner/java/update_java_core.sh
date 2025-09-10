#!/bin/bash

# Base URL
BASE_URL="https://central.sonatype.com/api/internal/browse/component/versions"

# Query parameters as an array
QUERY_PARAMS=(
    "sortField=normalizedVersion"
    "sortDirection=desc"
    "page=0"
    "size=12"
    "filter=namespace%3Acom.statsig%2Cname%3Ajavacore"
)

# Join query parameters with '&'
QUERY_STRING=$(IFS='&'; echo "${QUERY_PARAMS[*]}")

# Construct the full URL
FULL_URL="${BASE_URL}?${QUERY_STRING}"

RESULT=$(curl --location "$FULL_URL" --header 'accept: application/json')

LATEST_VERSION=$(echo $RESULT | jq -r '.components | map(select(.version | contains("-beta"))) | sort_by(.publishedEpochMillis) | .[-1].version')

echo "Latest beta version: ${LATEST_VERSION}"

# Replace all occurrences of 'com.statsig:javacore:+' with 'com.statsig:javacore:${LATEST_VERSION}' in build.gradle
sed -i  "s/com.statsig:javacore:+/com.statsig:javacore:${LATEST_VERSION}/g" build.gradle

echo "-------------------------------- BUILD GRADLE --------------------------------"
cat build.gradle