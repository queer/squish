#!/usr/bin/env bash

# 001-container-starts
# Assert that the container starts.

CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

if [ $CONTAINER_COUNT -ne 1 ]; then
  echo "Expected 1 container to be running, but found $CONTAINER_COUNT"
  exit 1
fi
