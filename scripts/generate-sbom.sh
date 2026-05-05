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
packages_file=$(mktemp)

cleanup() {
    rm -f "$packages_file"
}
trap cleanup EXIT

[ -n "$name" ] || fail "package name is missing"
[ -n "$version" ] || fail "package version is missing"

awk '
    function flush_package() {
        if (pkg_name != "" && pkg_version != "") {
            print pkg_name "\t" pkg_version "\t" pkg_source "\t" pkg_checksum
        }
    }

    /^\[\[package\]\]$/ {
        flush_package()
        pkg_name = ""
        pkg_version = ""
        pkg_source = ""
        pkg_checksum = ""
        next
    }

    /^name = / {
        value = $0
        sub(/^name = "/, "", value)
        sub(/"$/, "", value)
        pkg_name = value
        next
    }

    /^version = / {
        value = $0
        sub(/^version = "/, "", value)
        sub(/"$/, "", value)
        pkg_version = value
        next
    }

    /^source = / {
        value = $0
        sub(/^source = "/, "", value)
        sub(/"$/, "", value)
        pkg_source = value
        next
    }

    /^checksum = / {
        value = $0
        sub(/^checksum = "/, "", value)
        sub(/"$/, "", value)
        pkg_checksum = value
        next
    }

    END {
        flush_package()
    }
' Cargo.lock > "$packages_file"

json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

spdx_id() {
    printf 'SPDXRef-Package-%s-%s' "$1" "$2" | sed 's/[^A-Za-z0-9.-]/-/g'
}

write_cdx_components() {
    first=1
    while IFS="$(printf '\t')" read -r pkg_name pkg_version pkg_source pkg_checksum; do
        if [ "$pkg_name" = "$name" ] && [ "$pkg_version" = "$version" ]; then
            continue
        fi

        if [ "$first" -eq 0 ]; then
            printf ',\n'
        fi
        first=0

        escaped_name=$(json_escape "$pkg_name")
        escaped_version=$(json_escape "$pkg_version")
        escaped_source=$(json_escape "$pkg_source")
        escaped_checksum=$(json_escape "$pkg_checksum")
        purl=$(json_escape "pkg:cargo/$pkg_name@$pkg_version")

        cat <<EOF
    {
      "type": "library",
      "name": "$escaped_name",
      "version": "$escaped_version",
      "purl": "$purl",
      "externalReferences": [
        {
          "type": "distribution",
          "url": "$escaped_source"
        }
      ],
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "$escaped_checksum"
        }
      ]
    }
EOF
    done < "$packages_file"
}

write_spdx_packages() {
    first=1
    while IFS="$(printf '\t')" read -r pkg_name pkg_version pkg_source pkg_checksum; do
        if [ "$pkg_name" = "$name" ] && [ "$pkg_version" = "$version" ]; then
            continue
        fi

        if [ "$first" -eq 0 ]; then
            printf ',\n'
        fi
        first=0

        escaped_name=$(json_escape "$pkg_name")
        escaped_version=$(json_escape "$pkg_version")
        escaped_source=$(json_escape "$pkg_source")
        escaped_checksum=$(json_escape "$pkg_checksum")
        escaped_spdx_id=$(json_escape "$(spdx_id "$pkg_name" "$pkg_version")")
        purl=$(json_escape "pkg:cargo/$pkg_name@$pkg_version")

        cat <<EOF
    {
      "name": "$escaped_name",
      "SPDXID": "$escaped_spdx_id",
      "versionInfo": "$escaped_version",
      "downloadLocation": "NOASSERTION",
      "filesAnalyzed": false,
      "licenseDeclared": "NOASSERTION",
      "externalRefs": [
        {
          "referenceCategory": "PACKAGE-MANAGER",
          "referenceType": "purl",
          "referenceLocator": "$purl"
        },
        {
          "referenceCategory": "OTHER",
          "referenceType": "other",
          "referenceLocator": "$escaped_source"
        }
      ],
      "checksums": [
        {
          "algorithm": "SHA256",
          "checksumValue": "$escaped_checksum"
        }
      ]
    }
EOF
    done < "$packages_file"
}

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
  "components": [
$(write_cdx_components)
  ],
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
$(if [ "$(wc -l < "$packages_file" | tr -d ' ')" -gt 1 ]; then printf ',\n'; fi)
$(write_spdx_packages)
  ]
}
EOF

printf '%s\n' "generated artifacts/sbom/rust-supervisor.cdx.json"
printf '%s\n' "generated artifacts/sbom/rust-supervisor.spdx.json"
