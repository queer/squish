#!/usr/bin/env bash

# 004-other-alpine-versions-work
# Assert that default-uncached Alpine versions work.
# SQUISHFILE_OVERRIDE=./test/squishfiles/004-squishfile-3.13.toml

FILE_CONTENTS=$(curl -s -o- localhost:42069/alpine-release)
if [ "$FILE_CONTENTS" != "3.13.9" ]; then
  echo "Expected Alpine 3.13.9, got:\n$FILE_CONTENTS"
  echo -e "\n\n"
  echo "NOTE: This test can be flaky, as different IPs(?) seem to recv. different Alpine versions."
  exit 1
fi
