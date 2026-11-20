#!/usr/bin/env bash
# SPDX-FileCopyrightText: Maik-0000FF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Regenerates tests/conformance/expected: the results the reference
# implementation reports for every document of the conformance corpus.
#
# The reference is Saxon (Java) running the published rules: for EN 16931 the
# stylesheets the publisher ships, for XRechnung stylesheets compiled from the
# published Schematron with SchXslt. Everything is fetched at the versions and
# checksums pinned in sources.env.
#
# Needed only when the corpus or a rule set changes. The tests themselves need
# neither Java nor a network connection.
#
# The reference for the XML Schema is the schema processor of the JDK, run on
# the schema files of this repository.
#
# Requires: saxon-he, java (JDK 11 or later), cargo, curl, git, tar, unzip,
# sha256sum.
# Usage: tools/regenerate-conformance.sh

set -euo pipefail

# shellcheck source=tools/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=sources.env
source "$repository/sources.env"

readonly schxslt_pipeline="core/src/main/resources/xslt/2.0/pipeline-for-svrl.xsl"

# Runs a command of the reference tool.
reference_tool() {
  cargo run --quiet --release --manifest-path "$repository/Cargo.toml" \
    --package doc2einvoice-validation --example conformance_reference -- "$@"
}

# Runs the reference stylesheet over every exported case of one syntax. Saxon
# stops at a document it cannot evaluate a rule on; such a document simply has
# no complete report afterwards, which the collecting step records.
reference_run() {
  local key=$1 syntax=$2 stylesheet=$3
  mkdir -p "$work/reports/$key"
  saxon-he -s:"$work/cases/$syntax" -xsl:"$stylesheet" -o:"$work/reports/$key" \
    >/dev/null 2>"$work/$key.log" || true
  reference_tool collect "$key" "$work/reports/$key"
}

# Produces the expected verdicts of one XML Schema: first for the cases of the
# corpus, then for the variants of the cases the reference found valid.
schema_reference_run() {
  local key=$1 syntax=$2
  shift 2
  java "$repository/tools/SchemaReference.java" \
    "$work/cases/$syntax" "$work/$key-cases.json" "$@"
  reference_tool schema-variants "$key" "$work/$key-cases.json" "$work/variants/$key"
  java "$repository/tools/SchemaReference.java" \
    "$work/variants/$key" "$work/$key-variants.json" "$@"
  reference_tool schema-collect "$key" "$work/$key-cases.json" "$work/$key-variants.json"
}

echo "Fetching the pinned sources"
fetch "$EN16931_ARCHIVE_URL" "$EN16931_ARCHIVE_SHA256" "$work/en16931.tar.gz"
fetch "$XRECHNUNG_RULES_URL" "$XRECHNUNG_RULES_SHA256" "$work/xrechnung-rules.zip"
mkdir -p "$work/en16931" "$work/xrechnung"
tar --extract --gzip --file "$work/en16931.tar.gz" --directory "$work/en16931" --strip-components=1
unzip -q "$work/xrechnung-rules.zip" -d "$work/xrechnung"

git clone --quiet "$SCHXSLT_REPOSITORY" "$work/schxslt"
git -C "$work/schxslt" checkout --quiet "$SCHXSLT_COMMIT"

echo "Compiling the XRechnung reference stylesheets"
for syntax in ubl cii; do
  saxon-he -s:"$(find "$work/xrechnung/schematron/$syntax" -name '*.sch')" \
    -xsl:"$work/schxslt/$schxslt_pipeline" -o:"$work/xrechnung-$syntax.xsl"
done

echo "Exporting the corpus"
reference_tool export "$work/cases"

echo "Running the reference"
reference_run en16931-ubl ubl "$work/en16931/ubl/xslt/EN16931-UBL-validation.xslt"
reference_run en16931-cii cii "$work/en16931/cii/xslt/EN16931-CII-validation.xslt"
reference_run xrechnung-ubl ubl "$work/xrechnung-ubl.xsl"
reference_run xrechnung-cii cii "$work/xrechnung-cii.xsl"

echo "Running the XML Schema reference"
readonly schemas="$repository/schemas"
schema_reference_run ubl-2.1 ubl \
  "$schemas/ubl-2.1/maindoc/UBL-Invoice-2.1.xsd" \
  "$schemas/ubl-2.1/maindoc/UBL-CreditNote-2.1.xsd"
schema_reference_run cii-d16b cii \
  "$schemas/cii-d16b/CrossIndustryInvoice_100pD16B.xsd"
