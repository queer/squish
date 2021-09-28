#!/usr/bin/env bash

# 004-other-alpine-versions-work
# Assert that default-uncached Alpine versions work.
# SQUISHFILE_OVERRIDE=./test/squishfile-3.13.toml

FILE_CONTENTS=$(curl -s -o- localhost:42069/alpine-release)
if [ "$FILE_CONTENTS" != "3.13.6" ]; then
  echo "Expected Alpine 3.13.6, got:\n$FILE_CONTENTS"
  exit 1
fi
