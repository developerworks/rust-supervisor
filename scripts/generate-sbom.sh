#!/usr/bin/env sh
set -eu

fail() {
    printf '%s\n' "error: $1" >&2
    exit 1
}

[ -f Cargo.toml ] || fail "Cargo.toml is required"
[ -f Cargo.lock ] || fail "Cargo.lock is required"

mkdir -p artifacts/sbom

name=$(awk -F '"' '/^name = / { print $2; exit }' Cargo.toml)
version=$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)
repository=$(awk -F '"' '/^repository = / { print $2; exit }' Cargo.toml)
license=$(awk -F '"' '/^license = / { print $2; exit }' Cargo.toml)
lock_sha=$(cksum Cargo.lock | awk '{print $1 "-" $2}')
generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

[ -n "$name" ] || fail "package name is missing"
[ -n "$version" ] || fail "package version is missing"

cat > artifacts/sbom/rust-supervisor.cdx.json <<EOF
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.5",
  "serialNumber": "urn:uuid:00000000-0000-4000-8000-000000000001",
  "version": 1,
  "metadata": {
    "timestamp": "$generated_at",
    "tools": [
      {
        "vendor": "rust-supervisor",
        "name": "scripts/generate-sbom.sh",
        "version": "0.1.0"
      }
    ],
    "component": {
      "type": "library",
      "name": "$name",
      "version": "$version",
      "purl": "pkg:cargo/$name@$version",
      "licenses": [
        {
          "license": {
            "id": "$license"
          }
        }
      ],
      "externalReferences": [
        {
          "type": "vcs",
          "url": "$repository"
        }
      ]
    }
  },
  "components": [],
  "properties": [
    {
      "name": "cargo.lock.cksum",
      "value": "$lock_sha"
    }
  ]
}
EOF

cat > artifacts/sbom/rust-supervisor.spdx.json <<EOF
{
  "spdxVersion": "SPDX-2.3",
  "dataLicense": "CC0-1.0",
  "SPDXID": "SPDXRef-DOCUMENT",
  "name": "$name-$version",
  "documentNamespace": "https://github.com/developerworks/rust-supervisor/sbom/$version",
  "creationInfo": {
    "created": "$generated_at",
    "creators": [
      "Tool: scripts/generate-sbom.sh-0.1.0"
    ]
  },
  "packages": [
    {
      "name": "$name",
      "SPDXID": "SPDXRef-Package-rust-supervisor",
      "versionInfo": "$version",
      "downloadLocation": "$repository",
      "licenseDeclared": "$license",
      "externalRefs": [
        {
          "referenceCategory": "PACKAGE-MANAGER",
          "referenceType": "purl",
          "referenceLocator": "pkg:cargo/$name@$version"
        }
      ],
      "checksums": [
        {
          "algorithm": "OTHER",
          "checksumValue": "$lock_sha"
        }
      ]
    }
  ]
}
EOF

printf '%s\n' "generated artifacts/sbom/rust-supervisor.cdx.json"
printf '%s\n' "generated artifacts/sbom/rust-supervisor.spdx.json"
