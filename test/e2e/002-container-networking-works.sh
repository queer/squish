#!/usr/bin/env bash

# 002-container-networking-works
# Assert that a container can forward ports to the host as expected.

STATUS=$(curl -s -o /dev/null -w "%{http_code}" localhost:42069)
if [ "$STATUS" != "200" ]; then
  echo "Expected status code 200, got $STATUS"
  exit 1
fi