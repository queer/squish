#!/usr/bin/env bash

echo ">> Running quiet build..."
cargo -q build

echo ">> Starting tests!"
# Run daemon
cargo -q run -p daemon &
sleep 2
DAEMON=$(pidof daemon)

# Assert no containers running
CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

if [ $CONTAINER_COUNT -ne 0 ]; then
  echo "Expected 0 containers to be running, but found $CONTAINER_COUNT"
  exit 1
fi

# Start a container
cargo -q run -p cli -- create test/squishfile.toml > /dev/null

CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

if [ $CONTAINER_COUNT -ne 1 ]; then
  echo "Expected 1 container to be running, but found $CONTAINER_COUNT"
  exit 1
fi

# Stop the container
CONTAINER_ID=$(cargo -q run -p cli -- ps | grep -v "ID" | awk '{print $1}')
cargo -q run -p cli -- stop $CONTAINER_ID > /dev/null

# Assert no containers running
CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

if [ $CONTAINER_COUNT -ne 0 ]; then
  echo "Expected 0 containers to be running, but found $CONTAINER_COUNT"
  exit 1
fi

# Clean up
kill $DAEMON
echo ">> All good! :D"
