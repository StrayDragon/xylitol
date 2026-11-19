#!/bin/sh
# Smoke-task verifier: reward 1 only when /app/hello.txt is exactly "hello".
set -eu
mkdir -p /logs/verifier
if [ -f /app/hello.txt ] && [ "$(tr -d '\n' < /app/hello.txt)" = "hello" ]; then
  echo 1 > /logs/verifier/reward.txt
else
  echo 0 > /logs/verifier/reward.txt
fi
