#!/usr/bin/env bash

DEFAULT="\e[39m"
RED="\e[91m"
GREEN="\e[92m"

log() {
  TS=$(date +%T)
  echo -e "[$TS] $1"
}

start_container() {
  cargo -q run -p cli -- create test/squishfile.toml > /dev/null

  CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
  CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

  if [ $CONTAINER_COUNT -ne 1 ]; then
    log "Expected 1 container to be running, but found $CONTAINER_COUNT"
    exit 1
  fi
}

stop_container() {
  CONTAINER_ID=$(cargo -q run -p cli -- ps | grep -v "ID" | awk '{print $1}')
  cargo -q run -p cli -- stop $CONTAINER_ID > /dev/null

  CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
  CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

  if [ $CONTAINER_COUNT -ne 0 ]; then
    log "Expected 0 containers to be running, but found $CONTAINER_COUNT"
    exit 1
  fi
}

log "Running quiet build..."
cargo -q build

# Run daemon
cargo -q run -p daemon &
# Await daemon up
while [ "`curl -s -o /dev/null -w "%{http_code}" --unix-socket /tmp/squishd.sock http:/x/status`" != "200" ]; do
  sleep 0.5
done
sleep 1
log "Daemon up!"
DAEMON=$(pidof daemon)

log "Asserting sanity..."

# Assert no containers running
CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

if [ $CONTAINER_COUNT -ne 0 ]; then
  log "Expected 0 containers to be running, but found $CONTAINER_COUNT"
  exit 1
fi

log "Starting tests!"
# Run tests!
TOTAL=0
PASSED=0
for f in test/e2e/*.sh; do
  echo -e -n "[$(date +%T)] Running $f..."
  start_container
  OUTPUT=$(bash $f)
  LAST_STATUS=$?
  stop_container
  let TOTAL++
  if [ $LAST_STATUS -ne 0 ]; then
    echo -e "$RED FAILED$DEFAULT"
    echo -e $OUTPUT
    echo -e
  else
    let PASSED++
    echo -e "$GREEN PASSED$DEFAULT"
  fi
done

# Clean up
kill $DAEMON

if [ $PASSED -ne $TOTAL ]; then
  log "${RED}Failed $((TOTAL - PASSED))/$TOTAL tests!$DEFAULT"
  exit 1
else
  log "${GREEN}Passed $PASSED/$TOTAL tests!$DEFAULT"
fi

