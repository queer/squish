#!/usr/bin/env bash

DEFAULT="\e[39m"
RED="\e[91m"
GREEN="\e[92m"

log() {
  TS=$(date +%T)
  echo -e "[$TS] $1"
}

start_container() {
  cargo -q run -p cli -- create $1 > /dev/null

  CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
  CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

  if [ $CONTAINER_COUNT -ne 1 ]; then
    log "${RED}ERROR:${DEFAULT} Expected 1 container to be running, but found $CONTAINER_COUNT"
    exit 1
  fi
}

stop_container() {
  CONTAINER_ID=$(cargo -q run -p cli -- ps | grep -v "ID" | awk '{print $1}')
  cargo -q run -p cli -- stop $CONTAINER_ID > /dev/null

  CONTAINER_COUNT=$(cargo -q run -p cli -- ps | wc -l)
  CONTAINER_COUNT=$((CONTAINER_COUNT - 1))

  if [ $CONTAINER_COUNT -ne 0 ]; then
    log "${RED}ERROR:${DEFAULT} Expected 0 containers to be running, but found $CONTAINER_COUNT"
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
  SQUISHFILE=$(grep "SQUISHFILE_OVERRIDE=" $f | sed -e 's/# SQUISHFILE_OVERRIDE=//')
  SQUISHFILE="${SQUISHFILE:-./test/squishfiles/default.toml}"
  log "Starting container from: ${SQUISHFILE}"
  
  echo -e -n "[$(date +%T)] Running $(basename $f)..."

  start_container $SQUISHFILE
  START=$(($(date +%s%N)/1000000))
  OUTPUT=$(bash $f)
  LAST_STATUS=$?
  END=$(($(date +%s%N)/1000000))
  DURATION=$((END - START))
  stop_container
  let TOTAL++
  if [ $LAST_STATUS -ne 0 ]; then
    echo -e "${RED} FAILED${DEFAULT} (${DURATION}ms)"
    echo -e $OUTPUT
    echo -e
  else
    let PASSED++
    echo -e "${GREEN} PASSED${DEFAULT} (${DURATION}ms)"
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
