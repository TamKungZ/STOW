#!/usr/bin/env bash
set -euo pipefail

scripts=$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")
work=$(mktemp -d)
cd "$work"
passphrase=selftest-passphrase

generate=$work/generate
mkdir -m 700 "$generate"
GNUPGHOME=$generate gpg --batch --quiet --pinentry-mode loopback --passphrase "$passphrase" \
  --quick-generate-key "Selftest <selftest@example.invalid>" ed25519 sign never

export GPG_KEY
GPG_KEY=$(GNUPGHOME=$generate gpg --batch --pinentry-mode loopback --passphrase "$passphrase" \
  --armor --export-secret-keys)
export GPG_PASSPHRASE=$passphrase
export GITHUB_ENV=$work/env
: > "$GITHUB_ENV"
bash "$scripts/gpg-setup.sh"
set -a
. "$GITHUB_ENV"
set +a

echo "payload" > payload.txt
bash "$scripts/gpg-sign.sh" payload.txt

gpg --batch --armor --export "$GPG_KEY_ID" > public.asc
verify=$work/verify
mkdir -m 700 "$verify"
GNUPGHOME=$verify gpg --batch --quiet --import public.asc
GNUPGHOME=$verify gpg --batch --verify payload.txt.asc payload.txt

echo "tampered" > payload.txt
if GNUPGHOME=$verify gpg --batch --verify payload.txt.asc payload.txt 2> /dev/null; then
  echo "tampered file passed verification" >&2
  exit 1
fi

echo "gpg selftest ok"
