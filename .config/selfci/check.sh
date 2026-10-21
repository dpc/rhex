#!/usr/bin/env bash
# Shared gates for SelfCI and GitHub Actions; run inside nix develop.
set -eou pipefail

function step() {
  local name="$1"
  shift
  if [[ -n "${SELFCI_JOB_NAME:-}" ]]; then
    selfci step start "$name"
  else
    echo "==> $name"
  fi
  if ! "$@"; then
    if [[ -n "${SELFCI_JOB_NAME:-}" ]]; then
      selfci step fail
    fi
    return 1
  fi
}

function lint() {
  step "cargo fmt" nix build -L --no-link .#ci.cargoFmt
  step "treefmt" nix build -L --no-link .#treefmt
  step "Vale" just vale
}

function cargo_checks() {
  step "Cargo.lock up-to-date" cargo update --workspace --locked -q
  # Submit independent gates together; retain failed PTY diagnostics in the
  # Nix build directory, just as in the local SelfCI workflow.
  step "Nix cargo checks" nix build --keep-failed -L --no-link \
    .#ci.workspace .#default .#ci.clippy .#ci.tests
}

case "${1:-all}" in
  all) lint; cargo_checks ;;
  lint) lint ;;
  cargo) cargo_checks ;;
  *) echo "Unknown check group: $1" >&2; exit 1 ;;
esac
