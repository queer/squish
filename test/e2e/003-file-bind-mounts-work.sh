#!/usr/bin/env bash

# 003-file-bind-mounts-work
# Assert that a file bind-mounts into a container and can be read.

FILE_CONTENTS=$(curl -s -o- localhost:42069/Cargo.toml)
if [ "$FILE_CONTENTS" != "$(cat Cargo.toml)" ]; then
  echo "Expected Cargo.toml match, got:\n$FILE_CONTENTS"
  exit 1
fi
