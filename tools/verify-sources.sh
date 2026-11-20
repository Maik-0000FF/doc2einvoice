#!/usr/bin/env bash
# SPDX-FileCopyrightText: Maik-0000FF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Proves that the files of other publishers in this repository are the files
# those publishers released: downloads every archive pinned in sources.env,
# checks its checksum and compares the rule files, the XML Schema files, the
# test documents and the font files with it, byte for byte.
#
# The checksums stated in the code are computed from the files of this
# repository. This script is what ties those files to their publishers.
#
# Every directory of such files must be named below with the published
# directory it comes from. A directory that is not named fails the check, so
# files added later cannot go unverified.
#
# Requires: curl, tar, unzip, cmp, sha256sum, and a network connection.
# Usage: tools/verify-sources.sh

set -euo pipefail

# shellcheck source=tools/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=sources.env
source "$repository/sources.env"

readonly corpus="tests/conformance/corpus"
readonly fonts="src/lib/assets/fonts"

echo "Fetching the pinned archives"
fetch "$EN16931_ARCHIVE_URL" "$EN16931_ARCHIVE_SHA256" "$work/en16931.tar.gz"
fetch "$XRECHNUNG_RULES_URL" "$XRECHNUNG_RULES_SHA256" "$work/xrechnung-rules.zip"
fetch "$XRECHNUNG_TESTSUITE_URL" "$XRECHNUNG_TESTSUITE_SHA256" "$work/xrechnung-testsuite.zip"
fetch "$XML_SCHEMAS_URL" "$XML_SCHEMAS_SHA256" "$work/xml-schemas.zip"
fetch "$INTER_URL" "$INTER_SHA256" "$work/inter.zip"

mkdir -p "$work/en16931" "$work/xrechnung-rules" "$work/xrechnung-testsuite" "$work/xml-schemas" \
  "$work/inter"
tar --extract --gzip --file "$work/en16931.tar.gz" --directory "$work/en16931" --strip-components=1
unzip -q "$work/xrechnung-rules.zip" -d "$work/xrechnung-rules"
unzip -q "$work/xrechnung-testsuite.zip" -d "$work/xrechnung-testsuite"
unzip -q "$work/xml-schemas.zip" -d "$work/xml-schemas"
unzip -q "$work/inter.zip" -d "$work/inter"

# The two rule files of EN 16931 lie in different directories of the archive.
mkdir -p "$work/en16931-rules"
cp "$work/en16931/ubl/schematron/preprocessed/EN16931-UBL-validation-preprocessed.sch" \
  "$work/en16931/cii/schematron/preprocessed/EN16931-CII-validation-preprocessed.sch" \
  "$work/en16931-rules/"

# Each directory of this repository that holds files of another publisher, and
# the published directory those files come from.
declare -rA published=(
  ["rulesets/en16931"]="$work/en16931-rules"
  ["rulesets/xrechnung"]="$work/xrechnung-rules/schematron"
  ["schemas/ubl-2.1"]="$work/xml-schemas/resources/ubl/2.1/xsd"
  ["schemas/cii-d16b"]="$work/xml-schemas/resources/cii/16b/xsd"
  ["$corpus/en16931/unit-ubl-invoice"]="$work/en16931/test/Invoice-unit-UBL"
  ["$corpus/en16931/unit-ubl-creditnote"]="$work/en16931/test/CreditNote-unit-UBL"
  ["$corpus/en16931/unit-cii"]="$work/en16931/test/cii"
  ["$corpus/en16931/testfiles-ubl"]="$work/en16931/test/testfiles"
  ["$corpus/en16931/examples-ubl"]="$work/en16931/ubl/examples"
  ["$corpus/en16931/examples-cii"]="$work/en16931/cii/examples"
  ["$corpus/xrechnung/instances"]="$work/xrechnung-testsuite/instances"
  ["$fonts/inter"]="$work/inter/web"
)

# Compares every file below a directory of this repository with the file of
# the same relative path in the published directory. A published directory
# may hold more files than this repository uses.
same_as_published() {
  local directory=$1 file count=0
  local ours="$repository/$directory" theirs=${published[$directory]:-}
  if [ -z "$theirs" ]; then
    echo "no published directory is named for $directory" >&2
    return 1
  fi
  while IFS= read -r -d '' file; do
    cmp --silent "$file" "$theirs/${file#"$ours"/}" || {
      echo "differs from the published file: ${file#"$repository"/}" >&2
      return 1
    }
    count=$((count + 1))
  done < <(find "$ours" -type f -print0)
  echo "$directory: $count files are the published files"
}

# Checks every directory found at the given depth below a root, and refuses
# files that lie above that depth, outside any checked directory.
check_directories_below() {
  local root=$1 depth=$2 directory stray
  stray=$(find "$repository/$root" -maxdepth "$depth" -type f)
  if [ -n "$stray" ]; then
    echo "files outside a checked directory:" >&2
    echo "${stray//"$repository"\//}" >&2
    return 1
  fi
  while IFS= read -r directory; do
    same_as_published "${directory#"$repository"/}"
  done < <(find "$repository/$root" -mindepth "$depth" -maxdepth "$depth" -type d | sort)
}

echo "Rule files"
check_directories_below rulesets 1
echo "XML Schema files"
check_directories_below schemas 1
echo "Test documents"
check_directories_below "$corpus" 2
echo "Font files"
check_directories_below "$fonts" 1
