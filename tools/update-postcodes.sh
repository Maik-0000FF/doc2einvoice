#!/usr/bin/env bash
# SPDX-FileCopyrightText: Maik-0000FF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Rewrites crates/doc2einvoice-extract/data/postcodes.tsv from the postal
# codes GeoNames publishes (CC BY 4.0): for each country, its postal codes
# and the places they belong to, and nothing else. The extraction confirms
# with it that a line "postal code and place" is an address.
#
# GeoNames renews these files every day and keeps no earlier ones, so they
# cannot be pinned to a checksum the way sources.env pins other files. The
# script prints the checksum of every archive it read; with the date, they go
# into sources.env as the record of what the table was made from.
#
# Requires: curl, unzip, sort, sha256sum, and a network connection.
# Usage: tools/update-postcodes.sh

set -euo pipefail

# shellcheck source=tools/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=sources.env
source "$repository/sources.env"

readonly table="$repository/crates/doc2einvoice-extract/data/postcodes.tsv"
readonly header=$'country\tpostcode\tplace'

# The columns of a GeoNames postal code file: country, postal code, place.
readonly columns="1-3"

{
  echo "$header"
  for country in $GEONAMES_POSTCODE_COUNTRIES; do
    curl --silent --show-error --fail --location \
      --output "$work/$country.zip" "$GEONAMES_POSTCODES_URL/$country.zip"
    sha256sum "$work/$country.zip" | sed "s|$work/||" >&2
    unzip -q -p "$work/$country.zip" "$country.txt" | cut -f "$columns"
  done | LC_ALL=C sort -u
} >"$table"

echo "Wrote $(($(wc -l <"$table") - 1)) postal codes to ${table#"$repository"/}" >&2
