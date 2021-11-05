#!/bin/sh

set -e

echo -n "asdf" >> /app/scratch
/app/http-asm 2000 /app
