#!/usr/bin/env bash
# SPDX-FileCopyrightText: Maik-0000FF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Proves that every copy under vendor/ is the published crate with exactly its
# documented patch applied: downloads the crate, checks its checksum, applies
# the patch and compares the result with the copy.
#
# Requires: curl, tar, patch, diff, sha256sum, and a network connection.
# Usage: tools/verify-vendor.sh

set -euo pipefail

# shellcheck source=tools/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

readonly vendor="$repository/vendor"
readonly registry="https://static.crates.io/crates"
# Files cargo adds to a published crate that are not part of its source.
readonly packaging_files=(Cargo.lock .cargo_vcs_info.json .cargo-ok)

while read -r name version checksum patch_file; do
  case "$name" in '' | '#'*) continue ;; esac

  archive="$work/$name-$version.crate"
  fetch "$registry/$name/$name-$version.crate" "$checksum" "$archive"

  tar --extract --gzip --file "$archive" --directory "$work"
  published="$work/$name-$version"
  for file in "${packaging_files[@]}"; do
    rm -f "$published/$file"
  done
  patch --quiet --strip=1 --directory "$published" <"$vendor/patches/$patch_file"

  diff --recursive "$published" "$vendor/$name"
  echo "$name $version: the copy is the published crate plus $patch_file"
done <"$vendor/upstream.env"
