---
description: "Use when: user asks to commit code, push code, or submit changes. Commits current-theme files only, using conventional English commit message format, then pushes."
argument-hint: "[summary]"
agent: "agent"
---

Inspect the working tree, isolate files that belong to the current theme, draft a commit message in English, commit, and push.

## Commit Message Format (mandatory)

```
<summary line>
- <change item>
- <change item>
```

Rules:

- **Summary line**: one line, English, imperative mood, expresses a single theme. No multi-theme commits. Keep under 72 characters.
- **Change items**: flat bullet list, one per line. Describe what changed and the result. Do not mechanically list file names.
- Common summary prefixes: `Add`, `Fix`, `Refactor`, `Update`, `Remove`, `Consolidate`, `Wire`, `Implement`.

## Workflow

### 1. Inspect the working tree

Run:

```bash
git status --short
git diff --stat
```

Answer:

- What parallel changes exist in the working tree?
- Which files belong to the current theme?
- Which files must NOT enter this commit?

### 2. Isolate the path set

Only stage files that belong to the current theme. Typical scope:

- Source code + corresponding tests + spec/plan backfill → can go in one commit.
- Unrelated modules, unrelated skills, temporary files → exclude.

If the user provides a summary description after the slash command, use it as the commit summary line. Otherwise, derive the summary from the staged changes.

### 3. Draft the commit message

Write the commit message in the required format. Show it to the user for confirmation before committing.

### 4. Commit

```bash
git add <path-set>
git commit -m "<message>"
```

### 5. Push

```bash
git push
```

If the branch has no upstream, use:

```bash
git push --set-upstream origin <current-branch>
```
