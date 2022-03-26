#!/usr/bin/env bash

# 006-rw-file-binds-work
# Assert that a file bind-mounts into a container and can be read and written.
# SQUISHFILE_OVERRIDE=./test/squishfiles/006-squishfile-rw-mount.toml

rm ./test/support/006-scratch
touch ./test/support/006-scratch

FILE_CONTENTS=$(curl -s -o- localhost:42069/scratch)
if [ "$FILE_CONTENTS" != "asdf" ]; then
  echo "Expected 'asdf', got:\n$FILE_CONTENTS"
  exit 1
fi
