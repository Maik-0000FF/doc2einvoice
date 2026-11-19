# SPDX-FileCopyrightText: Maik-0000FF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Shared by the scripts in this directory; meant to be sourced, not run.
#
# Sets `repository` (the root of the repository) and `work` (a temporary
# directory that is removed when the script ends), and provides `fetch`.

# shellcheck shell=bash

# Read by the scripts that source this file.
# shellcheck disable=SC2034
repository=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
readonly repository

work=$(mktemp -d)
readonly work
trap 'rm -rf "$work"' EXIT

# Downloads a file and refuses it unless it has the given SHA-256 checksum.
fetch() {
  local url=$1 checksum=$2 target=$3
  curl --silent --show-error --fail --location --output "$target" "$url"
  echo "$checksum  $target" | sha256sum --check --quiet
}
