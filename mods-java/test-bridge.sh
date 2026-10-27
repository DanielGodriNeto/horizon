#!/bin/sh
# Bridge check: the gemjava mod must register `gemcount`, count a tick, a block break and the command.
# Needs the shim and mod built (build-shim.cmd) and p3-bench built; JAVA = a JDK 25 java.exe.
set -e
cd "$(dirname "$0")"; W=$(pwd -W)
: "${JAVA:?set JAVA to a JDK 25 java.exe}"
out=$(printf 'B 1 1 2 3 minecraft:stone\nT 2 200\nC 3 gemcount\n' | ./p3-bench/target/release/p3-bench.exe \
  --java "$JAVA" --sidecar-cp "$W/sidecar/out;$W/out-shim" --host-jar "$W/out-mod/gemjava.jar" --bridge)
echo "$out"
echo "$out" | grep -q '^K gemcount' || { echo "FAIL: command not registered"; exit 1; }
echo "$out" | grep -q '^R 1 gem 1 minecraft:stone' || { echo "FAIL: block break reply"; exit 1; }
echo "$out" | grep -q '^R 2 $' || { echo "FAIL: tick reply"; exit 1; }
echo "$out" | grep -q '^R 3 gem breaks=1 ticks=1' || { echo "FAIL: command reply"; exit 1; }
echo "bridge ok"
