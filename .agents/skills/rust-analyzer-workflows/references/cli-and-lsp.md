# rust-analyzer CLI and LSP Reference

## 1. Preferred CLI Commands

### Discover Which rust-analyzer You Are Actually Using

```bash
command -v rust-analyzer
rust-analyzer --version
rustup which rust-analyzer
```

Good for:

- distinguishing the shell-visible `rust-analyzer` from the current toolchain binary
- confirming the executable path before wiring VS Code to a custom server
- checking whether `rustup` is resolving to a different binary than your shell PATH suggests

Practical notes:

- on the current machine, `command -v rust-analyzer` resolved to `/root/.cargo/bin/rust-analyzer`
- on the current machine, `rustup which rust-analyzer` resolved to `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rust-analyzer`
- this is useful because the rust-analyzer VS Code extension bundles its own server by default unless `rust-analyzer.server.path` overrides it

### Syntax Tree Check

```bash
rust-analyzer parse --no-dump < path/to/file.rs
```

Good for:

- single-file syntax checks
- fast syntax confirmation without involving macro expansion
- lightweight validation when you do not want to trigger full-workspace analysis

### File Structure Outline

```bash
rust-analyzer symbols < path/to/file.rs
```

Good for:

- quickly inspecting the structs, enums, `impl` blocks, and method hierarchy in one file
- scanning a file outline without spinning up a full LSP request
- confirming which symbols in a file are actually navigable

### Semantic Highlight Inspection

```bash
rust-analyzer highlight < path/to/file.rs
```

Good for:

- checking semantic highlighting and semantic token classification results
- debugging whether strings, comments, methods, types, and other tokens are recognized correctly
- troubleshooting editor themes or highlighting configuration

### Workspace Diagnostics

```bash
rust-analyzer diagnostics . --disable-build-scripts --disable-proc-macros --severity error
```

Good for:

- semantic error scanning without running `cargo check`
- quickly seeing which hard errors remain after a refactor

Notes:

- the command may exit with a non-zero status code when errors are found
- but it still prints a full diagnostics list to standard output, so do not rely on the exit code alone

### Structured Search and Replace

```bash
rust-analyzer search '$a.foo($b)'
rust-analyzer ssr '$a.foo($b) ==>> bar($a, $b)'
```

Good for:

- batch AST matching
- avoiding brittle plain-text replacements

Practical notes:

- `search` may try to resolve certain free-function path patterns first; if the path cannot be resolved in the current search context, it may fail immediately
- in those cases, prefer more stable patterns:
  - method-call patterns such as `$handle.request_shutdown()`
  - structural patterns rather than free-function calls with explicit paths

### Cache Warming

```bash
rust-analyzer prime-caches . --disable-build-scripts --disable-proc-macros --num-threads 1
```

Good for:

- cold workspace warm-up before semantic navigation or bulk LSP requests
- reducing first-request latency for `definition`, `references`, `workspace/symbol`, and similar semantic queries
- preheating the crate graph without running project compilation commands

Practical notes:

- this command was validated in the current environment
- it is a good first step when LSP methods return early empty results on a large workspace

### Workspace Shape and Item Stats

```bash
rust-analyzer analysis-stats . --disable-build-scripts --disable-proc-macros --skip-inference
```

Good for:

- quickly estimating crate count, module count, item-tree volume, and other semantic workspace shape data
- low-risk batch inspection when you want more than `symbols` but less than a full compiler pass
- debugging whether rust-analyzer can walk the workspace successfully at all

Practical notes:

- `--skip-inference` was validated in the current environment
- full `analysis-stats` without `--skip-inference` hit the same `attached db` panic class as `unresolved-references`
- treat `--skip-inference` as the default safe mode unless you are explicitly probing rust-analyzer internals

### Semantic Index Export

```bash
rust-analyzer scip . --output /tmp/project.scip
rust-analyzer lsif . > /tmp/project.lsif
```

Good for:

- exporting a stable semantic index for offline inspection or downstream tooling
- validating that rust-analyzer can resolve enough of the workspace to emit cross-reference data
- integrating with tools that consume SCIP（语义索引协议） or LSIF（语言服务器索引格式）

Practical notes:

- both `scip` and `lsif` were validated in the current environment
- these commands are heavier than `symbols` or direct LSP requests, but still useful when the goal is index export rather than editor interaction

### Config Schema Export

```bash
rust-analyzer --print-config-schema
```

Good for:

- listing all `rust-analyzer.*` configuration options
- confirming the authoritative names for settings such as `completion`, `inlayHints`, `lens`, `typing`, and `semanticHighlighting`
- cross-checking editor configuration for VS Code, Zed, Neovim, and similar editors

### VS Code Bundled Server Discovery

```bash
setopt null_glob; print -l ~/.vscode/extensions/rust-lang.rust-analyzer-* ~/.vscode-server/extensions/rust-lang.rust-analyzer-*
setopt null_glob; print -l ~/.vscode/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer ~/.vscode-server/extensions/rust-lang.rust-analyzer-*/server/rust-analyzer
```

Good for:

- locating the extension-bundled server binary that VS Code would use by default
- distinguishing the extension binary from a toolchain-managed or custom-installed binary
- checking remote setups such as VS Code Server / WSL, where the extension often lives under `~/.vscode-server/extensions`

Practical notes:

- on the current machine, the installed remote extension directory was `/root/.vscode-server/extensions/rust-lang.rust-analyzer-0.3.2870-linux-x64`
- on the current machine, the bundled server binary was `/root/.vscode-server/extensions/rust-lang.rust-analyzer-0.3.2870-linux-x64/server/rust-analyzer`
- the official VS Code docs say the extension stores its server under an extension directory named `rust-lang.rust-analyzer-*`

### VS Code Toolchain Override

In VS Code settings, you can point the extension at a specific binary:

```json
{
  "rust-analyzer.server.path": "/absolute/path/to/rust-analyzer"
}
```

Good for:

- forcing VS Code to use the toolchain or a custom `rust-analyzer` instead of the bundled server
- ensuring the editor uses the same binary you validated on the command line
- pinning VS Code to a source build or custom download

Practical notes:

- the installed extension declares `rust-analyzer.server.path` with the description `Path to rust-analyzer executable (points to bundled binary by default).`
- a good default value is the output of `command -v rust-analyzer` or `rustup which rust-analyzer`
- the official VS Code docs also show this setting for custom or manually obtained servers

## 2. Direct LSP Requests

When the CLI is not enough, talk directly to the `rust-analyzer` LSP server.

A typical sequence is:

1. `initialize`
2. `initialized`
3. `textDocument/didOpen`
4. `textDocument/definition` / `textDocument/references` / `textDocument/prepareRename` / `textDocument/rename` / other requests

### Why `didOpen` Should Come First

In local testing, directly sending `textDocument/definition` for some files could return:

```text
file not found
```

After sending `didOpen` first, both `definition` and `references` returned normally.

### Why Warm-Up and Retry Matter

If you send semantic requests immediately after `didOpen`, you may see:

- `result: []`
- `content modified`

In local testing, semantic requests such as `definition` and `references` became much more stable when we:

- waited a few seconds for the server to finish indexing
- retried once or more on transient empty results or `content modified`

This is more reliable than issuing a single request immediately.

### Grouped by Capability

Navigation and structure:

- `textDocument/definition`
- `textDocument/references`
- `textDocument/typeDefinition`
- `textDocument/implementation`
- `workspace/symbol`
- `textDocument/documentSymbol`
- `textDocument/documentHighlight`
- `textDocument/selectionRange`
- `textDocument/foldingRange`
- `textDocument/prepareCallHierarchy`
- `callHierarchy/incomingCalls`
- `callHierarchy/outgoingCalls`

Code understanding and hints:

- `textDocument/hover`
- `textDocument/completion`
- `textDocument/signatureHelp`
- `textDocument/inlayHint`
- `textDocument/semanticTokens/full`
- `rust-analyzer/analyzerStatus`
- `rust-analyzer/expandMacro`
- `experimental/externalDocs`
- `rust-analyzer/viewSyntaxTree`
- `rust-analyzer/viewHir`
- `rust-analyzer/viewMir`
- `rust-analyzer/viewItemTree`
- `rust-analyzer/viewFileText`
- `rust-analyzer/getFailedObligations`
- `rust-analyzer/interpretFunction`
- `rust-analyzer/viewRecursiveMemoryLayout`

Local refactors and editing:

- `textDocument/prepareRename`
- `textDocument/rename`
- `textDocument/codeAction`
- `textDocument/formatting`

Workspace / project utilities:

- `experimental/parentModule`
- `experimental/runnables`
- `experimental/openCargoToml`
- `rust-analyzer/relatedTests`
- `rust-analyzer/rebuildProcMacros`
- `rust-analyzer/reloadWorkspace`

### Feature Flags and Capability Negotiation

Based on the official docs and local validation, these are the most useful details to remember:

- for richer `codeAction` results, advertise `codeActionLiteralSupport`
- for fuller auto-import flavored `completion`, advertise `completionItem.resolveSupport.properties = ["additionalTextEdits", ...]`
- `workspace/symbol` supports query syntax such as `#` and `*` in addition to ordinary text queries
- `workspace/symbol` also supports `searchScope` and `searchKind`; the bundled script exposes them as `--symbol-scope` and `--symbol-kind`
- `experimental/externalDocs` can return both web docs and local Rust toolchain docs when the client advertises `localDocs`
- once diagnostic pull support is declared, rust-analyzer may send `workspace/diagnostic/refresh`; the bundled script now acknowledges those client-directed requests before waiting for the actual response
- `rangeFormatting` is not enabled by default; the official docs say it depends on `rust-analyzer.rustfmt.rangeFormatting.enable`, and `rustfmt` range formatting itself is still somewhat unstable
- for VS Code version inspection, the official troubleshooting docs point to the `rust-analyzer: Show RA Version` command in the command palette

## 3. Behaviors Confirmed in This Discussion

Environment:

- `rust-analyzer 1.94.1 (e408947 2026-03-25)`

Verified working:

- `parse`
- `diagnostics`
- `symbols`
- `highlight`
- `prime-caches`
- `analysis-stats --skip-inference`
- `scip`
- `lsif`
- `--print-config-schema`
- `search`
- `ssr`
- direct LSP `initialize`
- direct LSP `definition`
- direct LSP `references`
- direct LSP `typeDefinition`
- direct LSP `implementation`
- direct LSP `workspace/symbol`
- direct LSP `documentHighlight`
- direct LSP `selectionRange`
- direct LSP `foldingRange`
- direct LSP `completion`
- direct LSP `signatureHelp`
- direct LSP `inlayHint`
- direct LSP `semanticTokens/full`
- direct LSP `prepareRename`
- direct LSP `rename`
- direct LSP `codeAction`
- direct LSP `formatting`
- direct LSP `prepareCallHierarchy`
- direct LSP `callHierarchy/incomingCalls`
- direct LSP `callHierarchy/outgoingCalls`
- direct LSP `rust-analyzer/analyzerStatus`
- direct LSP `rust-analyzer/expandMacro`
- direct LSP `experimental/externalDocs`
- direct LSP `experimental/parentModule`
- direct LSP `experimental/runnables`
- direct LSP `experimental/openCargoToml`
- direct LSP `rust-analyzer/relatedTests`
- direct LSP `rust-analyzer/viewSyntaxTree`
- direct LSP `rust-analyzer/viewHir`
- direct LSP `rust-analyzer/viewItemTree`
- direct LSP `rust-analyzer/viewMir`
- direct LSP `rust-analyzer/viewFileText`
- direct LSP `rust-analyzer/getFailedObligations`
- direct LSP `rust-analyzer/interpretFunction`
- direct LSP `rust-analyzer/viewRecursiveMemoryLayout`
- direct LSP `rust-analyzer/rebuildProcMacros`
- direct LSP `rust-analyzer/reloadWorkspace`
- filtered direct LSP `workspace/symbol`

Observed `rename` behavior:

- `prepareRename` returns the precise renameable range for the current symbol
- `rename` returns a `WorkspaceEdit` structure
- the current script only prints the result and does not automatically apply the rename to disk

Observed `completion` behavior:

- it returns both ordinary completions and snippet completions
- with the right client capabilities declared, you can see items that include import-related data
- the current script supports `--resolve-index <n>` to send `completionItem/resolve` for one completion item

Observed `codeAction` behavior:

- `--only refactor` is more likely to produce stable assist results
- `--only quickfix` is better for cases such as unused items, explicit types, and local fixes
- cursor position and selection range strongly affect the result; an empty result does not necessarily mean the feature is unavailable

Observed `callHierarchy` behavior:

- `prepareCallHierarchy` returns the function or method node at the current position
- `incomingCalls` is useful for checking who would be affected before a repository-wide rename
- `outgoingCalls` is useful for following dependencies along an execution path

Observed `expandMacro` behavior:

- it expands macro calls from the server's semantic view instead of guessing from source text
- local validation successfully expanded `vec![1, 2, 3]` into the lowered allocation form

Observed `externalDocs` behavior:

- it can return both a web URL and a local `file://` doc URL
- local validation against a standard-library symbol returned both rust-lang docs and the local toolchain HTML path

Observed `runnables` and `relatedTests` behavior:

- `runnables` returns rust-analyzer's own understanding of runnable cargo targets at the current location
- local validation returned module tests, a single test, `cargo check`, and `cargo test` runnable entries for the probe crate
- `relatedTests` can map a symbol location back to associated tests; local validation returned the expected smoke test

Observed semantic debug behavior:

- `viewHir` returns the lowered HIR for the function at the current position; local validation returned the expected `fn interpret_probe() -> i32 {...}` summary
- `viewMir` returns MIR text for the function at the current position; local validation returned the expected block structure for `interpret_probe`
- `getFailedObligations` is useful for trait-resolution debugging; local validation returned `Foo: Display` obligation failures on the probe crate
- `interpretFunction` successfully evaluated a simple pure function in local validation and returned a `pass` result
- `viewRecursiveMemoryLayout` returned structured size, offset, and alignment data for the probe `LayoutProbe` type, which makes it useful for layout and padding investigations

Observed project utility behavior:

- `openCargoToml` returned the expected `Cargo.toml` location from a source file position
- `rebuildProcMacros` succeeded and returned `null`, which is expected for a maintenance request without a payload

Observed VS Code extension behavior on the current machine:

- the installed remote extension exposed a `rust-analyzer.serverVersion` command titled `Show RA Version`
- the installed remote extension declared a `rust-analyzer.server.path` setting and described it as pointing to the bundled binary by default
- this confirms that a practical workflow is to compare `rust-analyzer --version` on the command line with `rust-analyzer: Show RA Version` in VS Code when you suspect binary mismatch

Observed debug / inspection behavior:

- `analyzerStatus` returns workspace, crate, and configuration status useful for diagnosing rust-analyzer itself
- `viewSyntaxTree`, `viewItemTree`, and `viewFileText` expose rust-analyzer's internal view and are useful when navigation results look suspicious
- `parentModule` returns the containing module edge and is useful when tracing file-to-module ownership
- `reloadWorkspace` succeeded and returned `null`, which is expected for a fire-and-forget refresh request

Observed `workspace/symbol` filtering behavior:

- local validation confirmed `searchScope = workspace` and `searchKind = onlyTypes`
- this is useful for limiting noisy results when probing large workspaces for type owners only

Observed experimental / weak results:

- `textDocument/diagnostic` no longer gets confused by `workspace/diagnostic/refresh`, but local validation still returned an empty `items` list on a probe file with a known unresolved reference
- `rust-analyzer/fetchDependencyList` returned an empty crate list in both the probe crate and the real repository
- `rust-analyzer/viewCrateGraph` returned an empty digraph in both the probe crate and the real repository, even with `full = true`

Treat those three as exposed for experiments, not as default recommended workflows

Observed issues:

- `unresolved-references` may panic on some workspaces
- full `analysis-stats` without `--skip-inference` may panic with the same internal `attached db` failure class
- `diagnostics` works and exposes real syntax and semantic errors
- direct LSP requests work in the current environment, but `didOpen` still matters
- `signatureHelp` is sensitive to cursor position; being off by one character can return `null`
- `formatting` may return `null` when the file is already formatted

## 3.1 When `rust-analyzer` Is Actually Faster for Structural Refactors

Practical conclusion:

- for tasks such as splitting `structs.rs` / `struct_impl.rs` into per-type files, `rust-analyzer` is best treated as a semantic refactor accelerator, not as a file-splitting robot
- the fastest workflow is usually hybrid: use `rust-analyzer` for semantic discovery and validation, then use file edits for the physical module rewrite

`rust-analyzer` is usually faster when the expensive part is understanding ownership and impact:

- enumerate which `struct`, `enum`, trait, and `impl` blocks are really present in a file
- confirm which callers, exports, or re-exports depend on a symbol before moving it
- perform safer type, module, or export renames
- run `diagnostics` after a refactor pass to surface semantic breakage quickly

It does not fully replace manual file-layout work:

- creating many new files
- rewriting `mod.rs`, `pub use`, and `#[path = "..."]`
- enforcing repository-specific owner and naming conventions
- maintaining external test mounts and temporary compatibility exports

Recommended workflow for this class of refactor:

1. use `rg` to find candidate `structs.rs` / `struct_impl.rs` files
2. use `symbols` or `documentSymbol` to enumerate types and `impl` blocks
3. use `definition`, `references`, `workspaceSymbol`, `rename`, or `codeAction` to understand the blast radius
4. use manual file edits to create per-type files and rewire the module facade
5. validate with `diagnostics`, then `cargo check` / `cargo test --no-run` as needed

Heuristic:

- if the hard question is “who owns this symbol?” or “who depends on this method?”, prefer `rust-analyzer`
- if the hard question is “how should these files be reorganized on disk?”, `rust-analyzer` helps, but manual patches still do the real work
- for large structural cleanups, the hybrid workflow is usually faster and safer than either pure text search or pure hand-editing

## 4. Recommended Fallback Paths

If `unresolved-references` is unstable:

1. use `rg` to search for leftover old paths
2. use `diagnostics` to inspect semantic errors
3. use LSP `definition` / `references` for the suspicious symbols

If raw stdin/tty bridging is unstable:

1. drive `rust-analyzer` through the script directly
2. do not depend on interactive terminal echo

## 5. Recommended Command Templates

### Workspace Symbol Search

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --method workspaceSymbol \
  --query SupervisedTaskHandle
```

### Filtered Workspace Symbol Search

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --method workspaceSymbol \
  --query Supervised \
  --symbol-scope workspace \
  --symbol-kind onlyTypes
```

### Completion Followed by Resolve

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 10 \
  --character 7 \
  --method completion \
  --resolve-index 0
```

### Code Actions

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 20 \
  --character 8 \
  --end-line 20 \
  --end-character 14 \
  --method codeAction \
  --only refactor
```

### Call Hierarchy

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 42 \
  --character 9 \
  --method incomingCalls
```

### Macro Expansion

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 26 \
  --character 7 \
  --method expandMacro
```

### External Docs

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 1 \
  --character 15 \
  --method externalDocs
```

### Runnables and Related Tests

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 34 \
  --character 7 \
  --method runnables

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 10 \
  --character 12 \
  --method relatedTests
```

### HIR and MIR Views

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 31 \
  --character 8 \
  --method viewHir

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 31 \
  --character 8 \
  --method viewMir
```

### Failed Obligations and Interpretation

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 36 \
  --character 18 \
  --method getFailedObligations

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 31 \
  --character 8 \
  --method interpretFunction
```

### Cargo.toml and Memory Layout

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 21 \
  --character 0 \
  --method openCargoToml

python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --line 43 \
  --character 31 \
  --method viewRecursiveMemoryLayout
```

### Analyzer Status

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --file src/lib.rs \
  --method analyzerStatus
```

### Proc-Macro Refresh

```bash
python3 {baseDir}/scripts/rust_analyzer_lsp.py \
  --workspace /path/to/repo \
  --method rebuildProcMacros
```

### VS Code Version Check

Use the command palette and run:

```text
rust-analyzer: Show RA Version
```

Practical notes:

- the official troubleshooting docs mention this exact command
- the documented way to open the command palette is `Ctrl+Shift+P`
- if you prefer `Ctrl+P`, type `>rust-analyzer: Show RA Version`
