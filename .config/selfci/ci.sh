#!/usr/bin/env bash
set -eou pipefail

case "$SELFCI_JOB_NAME" in
  main)
    selfci job start "lint"
    selfci job start "cargo"
    ;;
  lint|cargo)
    nix develop -c ./.config/selfci/check.sh "$SELFCI_JOB_NAME"
    ;;
  *)
    echo "Unknown job: $SELFCI_JOB_NAME" >&2
    exit 1
    ;;
esac
