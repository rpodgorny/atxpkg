#!/bin/sh
set -e -x

rm -rf target

cargo build --target x86_64-pc-windows-gnu --release --locked
