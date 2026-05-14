#!/usr/bin/env bash
set -euo pipefail

# Builds localized mdBook manuals into one static Pages artifact.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/target/mdbook"

command -v mdbook >/dev/null 2>&1 || {
  echo "mdbook is required" >&2
  exit 1
}

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}"
mkdir -p "${OUT_DIR}/docs"
cp "${ROOT_DIR}/docs/screenshot.png" "${OUT_DIR}/docs/screenshot.png"

mdbook build -d "${OUT_DIR}/en" "${ROOT_DIR}/manual/en"
mdbook build -d "${OUT_DIR}/zh" "${ROOT_DIR}/manual/zh"

cat >"${OUT_DIR}/index.html" <<'HTML'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>rust-supervisor Manual</title>
    <style>
      :root {
        color-scheme: light dark;
        font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }

      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        padding: 2rem;
      }

      main {
        max-width: 42rem;
      }

      h1 {
        margin: 0 0 0.75rem;
        font-size: 2rem;
      }

      p {
        margin: 0 0 1.5rem;
        line-height: 1.6;
      }

      nav {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
      }

      a {
        display: inline-flex;
        align-items: center;
        min-height: 2.5rem;
        padding: 0 1rem;
        border: 1px solid currentColor;
        border-radius: 0.375rem;
        color: inherit;
        text-decoration: none;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>rust-supervisor Manual</h1>
      <p>Select a language to read the project manual.</p>
      <nav aria-label="Manual language">
        <a href="./en/">English</a>
        <a href="./zh/">中文</a>
      </nav>
    </main>
  </body>
</html>
HTML

touch "${OUT_DIR}/.nojekyll"
