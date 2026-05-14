---
name: "speckit-specify"
description: "Create or update the feature specification from a natural language feature description."
metadata:
  compatibility: "Requires spec-kit project structure with .specify/ directory"
  author: "github-spec-kit"
  source: "templates/commands/specify.md"
---

## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty).

## Pre-Execution Checks

**Check for extension hooks (before specification)**:

- Check if `.specify/extensions.yml` exists in the project root.
- If it exists, read it and look for entries under the `hooks.before_specify` key
- If the YAML cannot be parsed or is invalid, skip hook checking silently and continue normally
- Filter out hooks where `enabled` is explicitly `false`. Treat hooks without an `enabled` field as enabled by default.
- For each remaining hook, do **not** attempt to interpret or evaluate hook `condition` expressions:
  - If the hook has no `condition` field, or it is null/empty, treat the hook as executable
  - If the hook defines a non-empty `condition`, skip the hook and leave condition evaluation to the HookExecutor implementation
- For each executable hook, output the following based on its `optional` flag:
  - **Optional hook** (`optional: true`):

    ```
    ## Extension Hooks

    **Optional Pre-Hook**: {extension}
    Command: `/{command}`
    Description: {description}

    Prompt: {prompt}
    To execute: `/{command}`
    ```

  - **Mandatory hook** (`optional: false`):

    ```
    ## Extension Hooks

    **Automatic Pre-Hook**: {extension}
    Executing: `/{command}`
    EXECUTE_COMMAND: {command}

    Wait for the result of the hook command before proceeding to the Outline.
    ```

- If no hooks are registered or `.specify/extensions.yml` does not exist, skip silently

## Outline

The text the user typed after `/speckit-specify` in the triggering message **is** the feature description. Assume you always have it available in this conversation even if `$ARGUMENTS` appears literally below. Do not ask the user to repeat it unless they provided an empty command.

Given that feature description, do this:

1. **Generate a concise short name** (2-4 words) for the feature:
   - Analyze the feature description and extract the most meaningful keywords
   - Create a 2-4 word short name that captures the essence of the feature
   - Use action-noun format when possible (e.g., "add-user-auth", "fix-payment-bug")
   - Preserve technical terms and acronyms (OAuth2, API, JWT, etc.)
   - Keep it concise but descriptive enough to understand the feature at a glance
   - Examples:
     - "I want to add user authentication" → "user-auth"
     - "Implement OAuth2 integration for the API" → "oauth2-api-integration"
     - "Create a dashboard for analytics" → "analytics-dashboard"
     - "Fix payment processing timeout bug" → "fix-payment-timeout"

2. **Functional requirement grouping before directory creation**:
   - Before creating any spec directory, extract the candidate `FR(Functional Requirement)` list from the user input.
   - Count one independently testable business behavior as one `FR(Functional Requirement)`.
   - Do not hide multiple business behaviors inside one `FR(Functional Requirement)` by using broad wording, parent requirements, combined verbs, or notes.
   - Acceptance criteria, assumptions, edge cases, and success criteria do not count as `FR(Functional Requirement)` unless they introduce a required business behavior.
   - Group related `FR(Functional Requirement)` items by Miller's Law(米勒定律), using exactly 3 `FR(Functional Requirement)` items as the fixed cognitive load target for each generated `spec.md`.
   - The fixed size for one spec is 3 `FR(Functional Requirement)` items. A final remainder spec may contain 1 or 2 `FR(Functional Requirement)` items only when fewer than 3 items remain.
   - If the candidate list contains 3 or fewer `FR(Functional Requirement)` items and the items are related, continue with one spec.
   - If the candidate list contains more than 3 `FR(Functional Requirement)` items, group related `FR(Functional Requirement)` items into separate spec slices before creating directories.
   - Preserve strongly related `FR(Functional Requirement)` items in the same spec slice when they describe the same business workflow, capability boundary, lifecycle phase, actor goal, or decision boundary.
   - Do not split one related group with 3 or fewer `FR(Functional Requirement)` items into separate specs only to create more specs, but do split it when it exceeds the 3 item upper bound.
   - Do not create one spec per `FR(Functional Requirement)` unless every `FR(Functional Requirement)` is genuinely independent from the others.
   - Preserve the user's business priority and workflow order inside and across spec slices.
   - If one related group contains more than 3 `FR(Functional Requirement)` items, split that group by a smaller business phase or sub-capability, and explain the dependency between the resulting specs.
   - Each spec slice must be independently plan-ready. It may reference earlier or later slices as dependencies, but it must not require hidden requirements from another slice.
   - When multiple spec slices come from the same user input or the same oversized related group, assign related spec numbers under one base feature number, for example `001-1`, `001-2`, `001-3`.
   - Example: 30 candidate `FR(Functional Requirement)` items for one governance workflow should produce related specs such as `001-1`, `001-2`, `001-3`, and `001-10`, with each spec holding a coherent 3 item group.
   - The 3 item upper bound is hard. Do not create one oversized spec and do not ask the user whether to split when the count is already above 3.

3. **Branch creation** (optional, via hook):

   If a `before_specify` hook ran successfully in the Pre-Execution Checks above, it will have created/switched to a git branch and output JSON containing `BRANCH_NAME` and `FEATURE_NUM`. Note these values for reference, but the branch name does **not** dictate the spec directory name.

   If the user explicitly provided `GIT_BRANCH_NAME`, pass it through to the hook so the branch script uses the exact value as the branch name (bypassing all prefix/suffix generation).

4. **Create the spec feature directories**:

   Specs live under the default `specs/` directory unless the user explicitly provides `SPECIFY_FEATURE_DIRECTORY` or `SPECIFY_FEATURE_DIRECTORIES`.

   **Resolution order for `SPECIFY_FEATURE_DIRECTORY`**:
   1. If the user explicitly provided `SPECIFY_FEATURE_DIRECTORIES` as an ordered list, use those paths as-is for multiple related specs
   2. If the user explicitly provided `SPECIFY_FEATURE_DIRECTORY` (e.g., via environment variable, argument, or configuration), use it as-is for one spec
      - If multiple related specs are required, treat `SPECIFY_FEATURE_DIRECTORY` as the base directory stem and derive related paths with ordered child suffixes, for example `specs/001-governance-1`, `specs/001-governance-2`
   3. Otherwise, auto-generate it under `specs/`:
      - Check `.specify/init-options.json` for `branch_numbering`
      - If `"timestamp"`: prefix is `YYYYMMDD-HHMMSS` (current timestamp)
      - If `"sequential"` or absent: prefix is `NNN` (next available 3-digit number after scanning existing directories in `specs/`)
      - For one spec, construct the directory name: `<prefix>-<short-name>` (e.g., `003-user-auth` or `20260319-143022-user-auth`)
      - For multiple related specs, reserve one base prefix for the user input before creating files:
        - If `"sequential"` or absent: use one shared base prefix plus an ordered child number, for example `001-1`, `001-2`, `001-3`, `001-4`
        - If `"timestamp"`: use one shared timestamp plus an ordered child number, for example `20260319-143022-1`, `20260319-143022-2`
      - For multiple related specs, construct each directory name from its related prefix and a slice-specific short name.
      - For multiple related specs, set `SPECIFY_FEATURE_DIRECTORIES` to the ordered list of resolved spec directories.
      - Set `SPECIFY_FEATURE_DIRECTORY` to the single resolved directory, or to the first resolved directory when multiple related specs are generated.

   **Create the directory and spec file**:
   - `mkdir -p SPECIFY_FEATURE_DIRECTORY`
   - Copy `.specify/templates/spec-template.md` to `SPECIFY_FEATURE_DIRECTORY/spec.md` as the starting point
   - Set `SPEC_FILE` to `SPECIFY_FEATURE_DIRECTORY/spec.md`
   - Persist the resolved path to `.specify/feature.json`:
     ```json
     {
       "feature_directory": "<resolved feature dir>"
     }
     ```
     Write the actual resolved directory path value (for example, `specs/003-user-auth`), not the literal string `SPECIFY_FEATURE_DIRECTORY`.
     This allows downstream commands (`/speckit-plan`, `/speckit-tasks`, etc.) to locate the feature directory without relying on git branch name conventions.
   - For multiple specs, repeat the directory and template-copy steps for every resolved directory.
   - For multiple specs, persist the ordered resolved paths to `.specify/feature.json`:
     ```json
     {
       "feature_directory": "<first resolved feature dir>",
       "active_feature_directory": "<first resolved feature dir>",
       "feature_directories": [
         "<first resolved feature dir>",
         "<second resolved feature dir>"
       ]
     }
     ```
     `active_feature_directory` is the first slice that should enter planning first. `feature_directories` is the complete ordered split result. Related split specs must keep their shared base number visible in these paths.

   **IMPORTANT**:
   - You must create one spec per related `FR(Functional Requirement)` group or sub-group.
   - Use Miller's Law(米勒定律) to keep each generated `spec.md` at the fixed 3 `FR(Functional Requirement)` size when possible.
   - You must not place more than 3 `FR(Functional Requirement)` items in any generated `spec.md`.
   - A single `/speckit-specify` invocation may create multiple related spec directories when the user input requires more than 3 `FR(Functional Requirement)` items.
   - The spec directory name and the git branch name are independent — they may be the same but that is the user's choice
   - The spec directories and files are always created by this command, never by the hook

5. Load `.specify/templates/spec-template.md` to understand required sections.

6. Follow this execution flow:
   1. Parse user description from arguments
      If empty: ERROR "No feature description provided"
   2. Extract key concepts from description
      Identify: actors, actions, data, constraints
   3. For unclear aspects:
      - Make informed guesses based on context and industry standards
      - Only mark with [NEEDS CLARIFICATION: specific question] if:
        - The choice significantly impacts feature scope or user experience
        - Multiple reasonable interpretations exist with different implications
        - No reasonable default exists
      - **LIMIT: Maximum 3 [NEEDS CLARIFICATION] markers total**
      - Prioritize clarifications by impact: scope > security/privacy > user experience > technical details
   4. Fill User Scenarios & Testing section
      If no clear user flow: ERROR "Cannot determine user scenarios"
   5. Generate Functional Requirements
      Each requirement must be testable
      Each spec slice should follow Miller's Law(米勒定律), with 3 functional requirements as the fixed target and maximum
      Do not merge unrelated requirements to force a 3 item group
      Use reasonable defaults for unspecified details (document assumptions in Assumptions section)
   6. Define Success Criteria
      Create measurable, technology-agnostic outcomes
      Include both quantitative metrics (time, performance, volume) and qualitative measures (user satisfaction, task completion)
      Each criterion must be verifiable without implementation details
   7. Identify Key Entities (if data involved)
   8. Return: SUCCESS (spec ready for planning)

7. Write each specification to its own `SPEC_FILE` using the template structure, replacing placeholders with concrete details derived from the feature description and that spec slice while preserving section order and headings.
   - For a single spec, write one `SPEC_FILE`.
   - For multiple specs, write one `SPEC_FILE` per slice and keep each slice's functional requirements, scenarios, edge cases, entities, assumptions, and success criteria scoped to that slice.

8. **Specification Quality Validation**: After writing the initial spec or specs, validate each spec against quality criteria:

   a. **Create Spec Quality Checklist**: Generate a checklist file at each `SPECIFY_FEATURE_DIRECTORY/checklists/requirements.md` using the checklist template structure with these validation items:

   ```markdown
   # Specification Quality Checklist: [FEATURE NAME]

   **Purpose**: Validate specification completeness and quality before proceeding to planning
   **Created**: [DATE]
   **Feature**: [Link to spec.md]

   ## Content Quality

   - [ ] No implementation details (languages, frameworks, APIs)
   - [ ] Focused on user value and business needs
   - [ ] Written for non-technical stakeholders
   - [ ] All mandatory sections completed

   ## Requirement Completeness

   - [ ] No [NEEDS CLARIFICATION] markers remain
   - [ ] Requirements are testable and unambiguous
   - [ ] Success criteria are measurable
   - [ ] Success criteria are technology-agnostic (no implementation details)
   - [ ] All acceptance scenarios are defined
   - [ ] Edge cases are identified
   - [ ] Scope is clearly bounded
   - [ ] Dependencies and assumptions identified

   ## Feature Readiness

   - [ ] All functional requirements have clear acceptance criteria
   - [ ] User scenarios cover primary flows
   - [ ] Feature meets measurable outcomes defined in Success Criteria
   - [ ] No implementation details leak into specification

   ## Notes

   - Items marked incomplete require spec updates before `/speckit-clarify` or `/speckit-plan`
   ```

   b. **Run Validation Check**: Review the spec against each checklist item:
   - For multiple specs, validate every generated spec independently
   - For each item, determine if it passes or fails
   - Document specific issues found (quote relevant spec sections)

   c. **Handle Validation Results**:
   - **If all items pass**: Mark checklist complete and proceed to reporting

   - **If items fail (excluding [NEEDS CLARIFICATION])**:
     1. List the failing items and specific issues
     2. Update the spec to address each issue
     3. Re-run validation until all items pass (max 3 iterations)
     4. If still failing after 3 iterations, document remaining issues in checklist notes and warn user

   - **If [NEEDS CLARIFICATION] markers remain**:
     1. Extract all [NEEDS CLARIFICATION: ...] markers from the spec
     2. **LIMIT CHECK**: If more than 3 markers exist, keep only the 3 most critical (by scope/security/UX impact) and make informed guesses for the rest
     3. For each clarification needed (max 3), present options to user in this format:

        ```markdown
        ## Question [N]: [Topic]

        **Context**: [Quote relevant spec section]

        **What we need to know**: [Specific question from NEEDS CLARIFICATION marker]

        **Suggested Answers**:

        | Option | Answer                    | Implications                          |
        | ------ | ------------------------- | ------------------------------------- |
        | A      | [First suggested answer]  | [What this means for the feature]     |
        | B      | [Second suggested answer] | [What this means for the feature]     |
        | C      | [Third suggested answer]  | [What this means for the feature]     |
        | Custom | Provide your own answer   | [Explain how to provide custom input] |

        **Your choice**: _[Wait for user response]_
        ```

     4. **CRITICAL - Table Formatting**: Ensure markdown tables are properly formatted:
        - Use consistent spacing with pipes aligned
        - Each cell should have spaces around content: `| Content |` not `|Content|`
        - Header separator must have at least 3 dashes: `|--------|`
        - Test that the table renders correctly in markdown preview
     5. Number questions sequentially (Q1, Q2, Q3 - max 3 total)
     6. Present all questions together before waiting for responses
     7. Wait for user to respond with their choices for all questions (e.g., "Q1: A, Q2: Custom - [details], Q3: B")
     8. Update the spec by replacing each [NEEDS CLARIFICATION] marker with the user's selected or provided answer
     9. Re-run validation after all clarifications are resolved

   d. **Update Checklist**: After each validation iteration, update the checklist file with current pass/fail status

9. **Report completion** to the user with:
   - For a single spec:
     - `SPECIFY_FEATURE_DIRECTORY` — the feature directory path
     - `SPEC_FILE` — the spec file path
   - For multiple specs:
     - `SPECIFY_FEATURE_DIRECTORIES` — the ordered feature directory paths
     - `SPEC_FILES` — the ordered spec file paths
     - `active_feature_directory` — the first spec that should enter planning first
   - Checklist results summary
   - Readiness for the next phase (`/speckit-clarify` or `/speckit-plan`)

10. **Check for extension hooks**: After reporting completion, check if `.specify/extensions.yml` exists in the project root.

- If it exists, read it and look for entries under the `hooks.after_specify` key
- If the YAML cannot be parsed or is invalid, skip hook checking silently and continue normally
- Filter out hooks where `enabled` is explicitly `false`. Treat hooks without an `enabled` field as enabled by default.
- For each remaining hook, do **not** attempt to interpret or evaluate hook `condition` expressions:
  - If the hook has no `condition` field, or it is null/empty, treat the hook as executable
  - If the hook defines a non-empty `condition`, skip the hook and leave condition evaluation to the HookExecutor implementation
- For each executable hook, output the following based on its `optional` flag:
  - **Optional hook** (`optional: true`):

    ```
    ## Extension Hooks

    **Optional Hook**: {extension}
    Command: `/{command}`
    Description: {description}

    Prompt: {prompt}
    To execute: `/{command}`
    ```

  - **Mandatory hook** (`optional: false`):

    ```
    ## Extension Hooks

    **Automatic Hook**: {extension}
    Executing: `/{command}`
    EXECUTE_COMMAND: {command}
    ```

- If no hooks are registered or `.specify/extensions.yml` does not exist, skip silently

**NOTE:** Branch creation is handled by the `before_specify` hook (git extension). Spec directory and file creation are always handled by this core command.

## Quick Guidelines

- Focus on **WHAT** users need and **WHY**.
- Avoid HOW to implement (no tech stack, APIs, code structure).
- Written for business stakeholders, not developers.
- DO NOT create any checklists that are embedded in the spec. That will be a separate command.

### Section Requirements

- **Mandatory sections**: Must be completed for every feature
- **Optional sections**: Include only when relevant to the feature
- When a section doesn't apply, remove it entirely (don't leave as "N/A")

### For AI Generation

When creating this spec from a user prompt:

1. **Make informed guesses**: Use context, industry standards, and common patterns to fill gaps
2. **Document assumptions**: Record reasonable defaults in the Assumptions section
3. **Limit clarifications**: Maximum 3 [NEEDS CLARIFICATION] markers - use only for critical decisions that:
   - Significantly impact feature scope or user experience
   - Have multiple reasonable interpretations with different implications
   - Lack any reasonable default
4. **Prioritize clarifications**: scope > security/privacy > user experience > technical details
5. **Think like a tester**: Every vague requirement should fail the "testable and unambiguous" checklist item
6. **Common areas needing clarification** (only if no reasonable default exists):
   - Feature scope and boundaries (include/exclude specific use cases)
   - User types and permissions (if multiple conflicting interpretations possible)
   - Security/compliance requirements (when legally/financially significant)

**Examples of reasonable defaults** (don't ask about these):

- Data retention: Industry-standard practices for the domain
- Performance targets: Standard web/mobile app expectations unless specified
- Error handling: User-friendly messages with appropriate fallbacks
- Authentication method: Standard session-based or OAuth2 for web apps
- Integration patterns: Use project-appropriate patterns (REST/GraphQL for web services, function calls for libraries, CLI args for tools, etc.)

### Success Criteria Guidelines

Success criteria must be:

1. **Measurable**: Include specific metrics (time, percentage, count, rate)
2. **Technology-agnostic**: No mention of frameworks, languages, databases, or tools
3. **User-focused**: Describe outcomes from user/business perspective, not system internals
4. **Verifiable**: Can be tested/validated without knowing implementation details

**Good examples**:

- "Users can complete checkout in under 3 minutes"
- "System supports 10,000 concurrent users"
- "95% of searches return results in under 1 second"
- "Task completion rate improves by 40%"

**Bad examples** (implementation-focused):

- "API response time is under 200ms" (too technical, use "Users see results instantly")
- "Database can handle 1000 TPS" (implementation detail, use user-facing metric)
- "React components render efficiently" (framework-specific)
- "Redis cache hit rate above 80%" (technology-specific)
