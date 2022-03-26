#!/bin/sh

set -e

echo $$ >> /app/scratch 2>&1
/app/http-asm 2000 /app
