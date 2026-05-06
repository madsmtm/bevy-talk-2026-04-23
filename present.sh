#!/bin/sh

set -euo pipefail

typst compile --input present=1 talk.typ present.pdf
nix-shell -p pympress --run "pympress present.pdf --talk-time 15"
