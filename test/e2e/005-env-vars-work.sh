#!/usr/bin/env bash

# 005-env-vars-work
# Assert that env var injection works
# SQUISHFILE_OVERRIDE=./test/squishfile-port-env.toml

FILE_CONTENTS=$(curl -s -o- localhost:42069/asdf)
if [ "$FILE_CONTENTS" != "ed4a92f174398375a5a036f5b06b61984d977640ff03e55288a5c62c7e8fcd0c" ]; then
  echo "Expected 'ed4a92f174398375a5a036f5b06b61984d977640ff03e55288a5c62c7e8fcd0c', got:\n$FILE_CONTENTS"
  exit 1
fi
