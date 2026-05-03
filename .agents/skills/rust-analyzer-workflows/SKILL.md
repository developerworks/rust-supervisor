---
name: rust-analyzer-workflows
description: Use when Codex needs IDE-like Rust semantic navigation, code intelligence, structured refactors, lightweight workspace diagnostics, or rust-analyzer-specific debug/inspection workflows. Covers stable CLI workflows (`parse`, `diagnostics`, `symbols`, `highlight`, `search`, `ssr`, `prime-caches`, `analysis-stats --skip-inference`, `lsif`, `scip`, `--print-config-schema`), direct LSP requests such as `definition`, `references`, `workspaceSymbol`, `completion`, `inlayHint`, `codeAction`, `rename`, `analyzerStatus`, `expandMacro`, `externalDocs`, `runnables`, `viewHir`, `viewMir`, `interpretFunction`, `openCargoToml`, `viewRecursiveMemoryLayout`, and known failure modes such as `unresolved-references` and full `analysis-stats` panicking on some workspaces.
---

# Rust Analyzer Workflows

## Overview

Use `rust-analyzer` as the default first tool for Rust codebase exploration and refactors. If a standalone `rust-analyzer` is missing, first try to install it for the active toolchain, then fall back to `rg` only when installation is not possible, fails, or the task is explicitly about exact text literals, paths, comments, or non-Rust files.

This skill was cross-checked against the official rust-analyzer book, especially:

- `book/contributing/lsp-extensions.html`
- `book/features.html`
- `book/configuration`
- `book/diagnostics.html`
- `book/troubleshooting.html`

Prefer the lightest workflow that answers the question:

- Syntax only: use `parse`
- Workspace diagnostics: use `diagnostics`
- Single-file structure or semantic coloring: use `symbols` / `highlight`
- Structured search/replace: use `search` / `ssr`
- Cache warming, index export, or workspace-scale stats: use `prime-caches`, `scip`, `lsif`, or `analysis-stats --skip-inference`
- IDE-like navigation, editor actions, and rust-analyzer debug requests: use direct LSP requests or the bundled script

Default bias:

- Start with a `rust-analyzer` workflow first
- If standalone `rust-analyzer` is missing, try to install it before changing tools
- Drop to `rg` only when you need literal-text grep semantics or a fallback around a rust-analyzer failure mode

## Quick Start

Use these CLI commands first:

```bash
rust-analyzer --version
rust-analyzer parse --no-dump < path/to/file.rs
rust-analyzer diagnostics . --disable-build-scripts --disable-proc-macros --severity error
rust-analyzer symbols < path/to/file.rs
rust-analyzer highlight < path/to/file.rs
rust-analyzer prime-caches . --disable-build-scripts --disable-proc-macros
rust-analyzer analysis-stats . --disable-build-scripts --disable-proc-macros --skip-inference
rust-analyzer scip . --output /tmp/project.scip
rust-analyzer lsif . > /tmp/project.lsif
rust-analyzer --print-config-schema
rust-analyzer search '$a.foo($b)'
rust-analyzer ssr '$a.foo($b) ==>> bar($a, $b)'
```

For LSP-style requests, use `{baseDir}/scripts/rust_analyzer_lsp.py`.

## Workflow

### 1. Confirm tool availability

Run:

```bash
command -v rust-analyzer
rust-analyzer --version
rust-analyzer --help
rustup which rust-analyzer
```

Do this before relying on CLI subcommands or LSP behavior.

If the shell command is missing or `rustup` reports an unknown `rust-analyzer` binary, do not immediately give up on semantic tooling. Prefer this order:

1. verify whether a standalone toolchain binary already exists via `rustup which rust-analyzer`
2. if it is genuinely missing, try to install it for the active toolchain with `rustup component add rust-analyzer --toolchain <active-toolchain>`
3. only after install is impossible or fails, fall back to `rg`, source reading, or other non-VS-Code tools

If VS Code is involved, distinguish three different things:

- the shell-visible executable from `command -v rust-analyzer`
- the current toolchain binary from `rustup which rust-analyzer`
- the VS Code extension bundled server under `~/.vscode/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer` or `~/.vscode-server/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer`

If you want VS Code to use the toolchain or a custom binary instead of the bundled server, set `rust-analyzer.server.path` to an explicit executable path, usually the output of `command -v rust-analyzer` or `rustup which rust-analyzer`.

For version discovery:

- Command line: run `rust-analyzer --version`
- VS Code command palette: official docs point to `rust-analyzer: Show RA Version`
- In VS Code, `Ctrl+Shift+P` is the documented way to open the command palette
- If you prefer `Ctrl+P`, type `>rust-analyzer: Show RA Version`

### 2. Use CLI before raw LSP when possible

Prefer:

- `parse --no-dump` for syntax checks on a specific file
- `diagnostics` for workspace-level semantic errors
- `symbols` for a fast outline of one file
- `highlight` for semantic token / highlighting inspection
- `prime-caches` when semantic requests are timing out or returning early empty results on a cold workspace
- `analysis-stats --skip-inference` for workspace shape, item-tree volume, and low-risk batch analysis
- `scip` / `lsif` when you need an exported semantic index rather than an editor-style answer
- `--print-config-schema` for the authoritative list of `rust-analyzer.*` settings
- `search` for structured AST matching
- `ssr` for structured replacement

Read `references/cli-and-lsp.md` when you need command templates or current behavior notes.

### 3. Use direct LSP for IDE-like navigation

Use raw LSP or the bundled script when you need:

- `definition`
- `references`
- `typeDefinition`
- `implementation`
- `workspaceSymbol`
- `workspaceSymbol` with scope / kind filtering
- `documentSymbol`
- `documentHighlight`
- `selectionRange`
- `foldingRange`
- `hover`
- `completion`
- `signatureHelp`
- `inlayHint`
- `semanticTokensFull`
- `prepareRename`
- `rename`
- `codeAction`
- `formatting`
- `prepareCallHierarchy`
- `incomingCalls`
- `outgoingCalls`
- `analyzerStatus`
- `expandMacro`
- `externalDocs`
- `parentModule`
- `runnables`
- `relatedTests`
- `viewSyntaxTree`
- `viewHir`
- `viewMir`
- `viewItemTree`
- `viewFileText`
- `getFailedObligations`
- `interpretFunction`
- `openCargoToml`
- `viewRecursiveMemoryLayout`
- `reloadWorkspace`

For text-document methods, always send `didOpen` before the request. In practice this avoided `file not found` results during local testing.
Also allow a short warm-up window before semantic requests. During local testing, semantic requests became stable after a few seconds of indexing plus retry on transient empty or `content modified` responses.

Use:

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 10 \
  --character 5 \
  --method definition
```

Useful extra examples:

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --method workspaceSymbol \
  --query SupervisedTaskHandle

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 20 \
  --character 12 \
  --method codeAction \
  --only refactor

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 42 \
  --character 9 \
  --method incomingCalls

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 26 \
  --character 7 \
  --method expandMacro

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 1 \
  --character 15 \
  --method externalDocs

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 34 \
  --character 7 \
  --method runnables

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --method workspaceSymbol \
  --query Supervised \
  --symbol-scope workspace \
  --symbol-kind onlyTypes

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 31 \
  --character 8 \
  --method viewHir

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 43 \
  --character 31 \
  --method viewRecursiveMemoryLayout
```

Useful VS Code / binary discovery commands:

```bash
command -v rust-analyzer
rust-analyzer --version
rustup which rust-analyzer
setopt null_glob; print -l ~/.vscode/extensions/rust-lang.rust-analyzer-* ~/.vscode-server/extensions/rust-lang.rust-analyzer-*
setopt null_glob; print -l ~/.vscode/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer ~/.vscode-server/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer
```

### 4. Know the capability-gated features

- Richer `codeAction` results depend on the client advertising code-action literal support.
- Auto-import flavored `completion` results are better when the client advertises completion resolve support for `additionalTextEdits`.
- The bundled script now declares those capabilities and can optionally run `completionItem/resolve` with `--resolve-index`.
- `externalDocs` can return both web and local docs URLs when the client advertises `localDocs`.
- Enabling diagnostic pull support means rust-analyzer may send back client-directed refresh requests such as `workspace/diagnostic/refresh`; the bundled script now acknowledges those requests instead of mistaking them for normal responses.
- `formatting` is easy to drive over raw LSP, but `rangeFormatting` is still separate and depends on `rust-analyzer.rustfmt.rangeFormatting.enable`.

### 5. Treat `unresolved-references` as opportunistic

`rust-analyzer unresolved-references` is useful in theory for post-refactor scans, but it is not as stable as `diagnostics`.

If it panics or crashes:

- fall back to targeted `rg`
- run `diagnostics`
- use direct `definition` / `references` queries for the suspicious symbols

Do not block the task on `unresolved-references`.

`analysis-stats` also needs a caution:

- In the current validated environment, `analysis-stats --skip-inference` works.
- Full `analysis-stats` with inference enabled can still hit the same `attached db` panic class as `unresolved-references`.
- Prefer `--skip-inference` unless you are deliberately probing rust-analyzer internals.

## Decision Rules

- Use `rust-analyzer` first for Rust discovery, navigation, and refactor planning.
- Use `workspaceSymbol`, `symbols`, `documentSymbol`, `definition`, `references`, `search`, or direct LSP before broad manual file-walking.
- If standalone `rust-analyzer` is missing, try to install it for the active toolchain before switching to text-only tooling.
- Use `rg` only when the task is primarily about exact text matching, path discovery, comments/strings, non-Rust assets, or when rust-analyzer has already proven unreliable for the specific query and installation/repair is not the right next step.
- Use `diagnostics` instead of `cargo check` when the user has asked to avoid compilation-heavy validation.
- Use `symbols` and `workspaceSymbol` before wide manual file-walking when you need a semantic outline.
- Use `ssr` before hand-editing repetitive Rust refactors.
- Use `codeAction` for local assists and quick fixes before hand-editing syntax-only rewrites.
- Use call hierarchy workflows before large rename / ownership refactors that may ripple across modules.
- Use `expandMacro` before guessing what a macro-generated item looks like.
- Use `runnables` and `relatedTests` when you need rust-analyzer's own understanding of what can be run or which tests cover a symbol.
- Use `analyzerStatus`, `viewSyntaxTree`, `viewHir`, `viewMir`, `viewItemTree`, `viewFileText`, or `getFailedObligations` when ordinary navigation works but the server's internal semantic view is suspicious.
- Use `interpretFunction` for tiny pure helper functions when you want a quick semantic sanity check without compiling the workspace.
- Use `viewRecursiveMemoryLayout` when layout, padding, or field ordering matters for performance work.
- Use `openCargoToml` when you need fast crate-root navigation from a source file.
- Use `rust-analyzer.server.path` when you need VS Code to use a known toolchain or custom server binary rather than the extension-bundled executable.
- Use direct LSP when the user explicitly wants IDE-like behavior.

## Practical Refactor Heuristics

For repository refactors, `rust-analyzer` should be the default discovery layer. Treat `rg` as a supporting raw-text tool, not the primary entry point.

Use it when the expensive part of the task is understanding the code rather than rewriting the filesystem:

- enumerate which `struct`, `enum`, trait, and `impl` blocks actually exist in a file
- confirm which callers, exports, or re-exports depend on a symbol before moving it
- perform safer symbol renames or inspect available code actions
- run lightweight semantic diagnostics after a refactor pass

Do not expect it to fully replace manual file-layout work:

- creating many new files
- rewriting `mod.rs`, `pub use`, or `#[path = "..."]` wiring
- physically splitting `structs.rs` / `struct_impl.rs` into per-type files
- enforcing repository-specific naming and owner conventions

For tasks like splitting `structs.rs` / `struct_impl.rs`, the fastest pattern is usually hybrid:

1. use `workspaceSymbol`, `symbols`, or `documentSymbol` to find candidate files, types, and `impl` blocks
2. use `definition`, `references`, `workspaceSymbol`, `rename`, or `codeAction` to check the semantic blast radius
3. use `rg` only as a supporting pass for exact literals, module wiring strings, macro callsites, or non-Rust files that rust-analyzer will not model well
4. use file edits to create the new owner files and rewire the module facade
5. validate with `diagnostics`, then `cargo check` / `cargo test --no-run` when needed

Heuristic:

- if the hard part is “what owns this symbol?” or “who depends on this method?”, start with `rust-analyzer`
- if the hard part is “how should these files be reorganized?”, use `rust-analyzer` first to understand ownership, then patch files manually
- if the hard part is “where does this exact literal/path/comment appear?”, use `rg` as a narrow supporting tool
- for large structural cleanups, the hybrid workflow is usually faster and safer than either pure rust-analyzer or pure text search alone

## Resources

### `{baseDir}/scripts/rust_analyzer_lsp.py`

Run a single `rust-analyzer` LSP request against a workspace and print the JSON result.

Supported methods:

- `definition`
- `references`
- `diagnostic`
- `typeDefinition`
- `implementation`
- `workspaceSymbol`
- `documentSymbol`
- `documentHighlight`
- `selectionRange`
- `foldingRange`
- `hover`
- `completion`
- `signatureHelp`
- `inlayHint`
- `semanticTokensFull`
- `prepareRename`
- `rename`
- `codeAction`
- `formatting`
- `prepareCallHierarchy`
- `incomingCalls`
- `outgoingCalls`
- `analyzerStatus`
- `expandMacro`
- `externalDocs`
- `getFailedObligations`
- `interpretFunction`
- `openCargoToml`
- `parentModule`
- `runnables`
- `relatedTests`
- `viewSyntaxTree`
- `viewHir`
- `viewItemTree`
- `viewMir`
- `viewFileText`
- `viewRecursiveMemoryLayout`
- `fetchDependencyList`
- `viewCrateGraph`
- `rebuildProcMacros`
- `reloadWorkspace`

The script already applies a conservative default:

- wait `3000ms` after `didOpen`
- retry transient empty or `content modified` semantic responses
- acknowledge server-initiated requests such as `workspace/diagnostic/refresh`

For `rename`, pass `--new-name <identifier>`. The script only returns the
`WorkspaceEdit` result and does not automatically write the rename to disk.
For `workspaceSymbol`, `--file` is optional and can be omitted.
For `workspaceSymbol`, you can further narrow the results with `--symbol-scope workspace|workspaceAndDependencies` and `--symbol-kind onlyTypes|allSymbols`.
For `completion`, you can pass `--resolve-index <n>` to also run `completionItem/resolve`.
For `codeAction`, pass one or more `--only ...` filters such as `refactor` or `quickfix`.
For `incomingCalls` / `outgoingCalls`, the script first runs `prepareCallHierarchy` at the given position and then issues the corresponding hierarchy request.
For `viewCrateGraph`, pass `--full-graph` to include dependency and sysroot crates.
`diagnostic`, `fetchDependencyList`, and `viewCrateGraph` are currently exposed for experiments, but they are not promoted as default workflows until they produce consistently useful results in the current environment.
`viewHir`, `viewMir`, `getFailedObligations`, `interpretFunction`, `openCargoToml`, `viewRecursiveMemoryLayout`, and `rebuildProcMacros` were validated locally after checking the official LSP extension docs.

### `references/cli-and-lsp.md`

Read this when you need:

- current command recommendations
- known limitations
- an explanation of why `didOpen` matters
- VS Code binary-selection guidance
- a summary of the observed behavior from the repository discussion that motivated this skill
