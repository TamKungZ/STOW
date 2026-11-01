#!/usr/bin/env bash
set -euo pipefail

: "${GPG_KEY:?GPG_KEY is not set}"

home=$(mktemp -d)
chmod 700 "$home"
export GNUPGHOME=$home
echo "allow-loopback-pinentry" > "$home/gpg-agent.conf"
echo "pinentry-mode loopback" > "$home/gpg.conf"
printf '%s' "${GPG_PASSPHRASE-}" | tr -d '\r\n' > "$home/passphrase"
chmod 600 "$home/passphrase"

printf '%s\n' "$GPG_KEY" | gpg --batch --quiet --import

fingerprints=$(gpg --batch --list-secret-keys --with-colons | awk -F: '$1 == "fpr" { print $10 }')
if [ -n "${EXPECTED_FPR-}" ]; then
  if ! grep -qx "$EXPECTED_FPR" <<< "$fingerprints"; then
    echo "imported key does not contain $EXPECTED_FPR" >&2
    exit 1
  fi
  key_id=$EXPECTED_FPR
else
  key_id=$(head -n 1 <<< "$fingerprints")
fi

if ! echo check | gpg --batch --pinentry-mode loopback --passphrase-file "$home/passphrase" \
  --local-user "$key_id" --clearsign > /dev/null 2> "$home/check.err"; then
  echo "key $key_id cannot sign: $(tail -n 1 "$home/check.err")" >&2
  if [ ! -s "$home/passphrase" ]; then
    echo "GPG_PASSPHRASE is empty or not set" >&2
  fi
  exit 1
fi

if [ -n "${GITHUB_ENV-}" ]; then
  echo "GNUPGHOME=$home" >> "$GITHUB_ENV"
  echo "GPG_KEY_ID=$key_id" >> "$GITHUB_ENV"
else
  echo "export GNUPGHOME=$home GPG_KEY_ID=$key_id"
fi
