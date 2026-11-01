#!/usr/bin/env bash
set -euo pipefail

: "${GNUPGHOME:?run gpg-setup.sh first}"
: "${GPG_KEY_ID:?run gpg-setup.sh first}"

for file in "$@"; do
  gpg --batch --yes --pinentry-mode loopback --passphrase-file "$GNUPGHOME/passphrase" \
    --local-user "$GPG_KEY_ID" --armor --detach-sign --output "$file.asc" "$file"
  gpg --batch --verify "$file.asc" "$file"
done
