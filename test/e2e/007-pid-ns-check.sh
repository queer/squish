#!/usr/bin/env bash

# 007-pid-ns-check
# Ensure that we actually entire a new pid namespace
# SQUISHFILE_OVERRIDE=./test/squishfiles/007-squishfile-pid-ns.toml

rm ./test/support/007-scratch
touch ./test/support/007-scratch

FILE_CONTENTS=$(curl -s -o- localhost:42069/scratch)
if [ "$FILE_CONTENTS" != "2" ]; then
  echo "Expected '2', got:\n$FILE_CONTENTS"
  exit 1
fi