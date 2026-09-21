# RustGames Agent Checklist

Applies to all Rust game projects in this workspace. `CODE_STANDARDS.md` is the detailed code authority; `UI_STYLE.md` governs game UI composition and visual review; `MACROQUAD_TOOLKIT.md` and `GAME_DEVELOPMENT_GUIDE.md` provide API examples and setup guidance. Edit shared documents in `rust_management/docs/`, then distribute them with its sync script; keep project-specific guidance in the project's README or `PROJECT_AGENTS.md`.

## Implementation

- Use Rust, `macroquad`, and `macroquad-toolkit` by default. Consider missing shared capabilities as toolkit upgrades before adding local alternatives; diverge only for an established project pattern or a game-specific need.
- Follow `CODE_STANDARDS.md`: cohesive modules, named module files (no new `mod.rs`), explicit state ownership, UI actions, clear errors, and no unused code.
- Keep every `.rs` file within the 800-total-line hard limit, with no exemptions; follow §2.2 for counting and restructuring.
- Load JSON game data through the toolkit; projects own schemas and semantic validation (§5.3).
- Make browser gameplay fully touch-accessible, with visible controls and explicit tutorial instructions (§7.5).
- Read `UI_STYLE.md` before designing or changing screens. Plan the current decision and dominant play area, emphasize relevant actions, and defer secondary information. Simplify existing screens before adding panels; recompose the template's demo UI for each new game.
- Keep gameplay deterministic where practical; isolate randomness in small helpers or state-owned RNG.
- Match existing style, avoid unrelated refactors, and add dependencies only when they remove real complexity or match an established pattern.
- Keep a root `catalog_thumbnail.png` for publishing (§8.5).

## Shared builds

- Use `..\rust_management\cargo.ps1` for local build, check, test, Clippy and run commands from a game directory. Publishing and the shared capture wrapper acquire the same three-slot pool automatically. Formatting can use ordinary Cargo. In PowerShell quote the argument separator: `cargo.ps1 clippy '--' -D warnings` or `cargo.ps1 test '--' --nocapture`.
- Do not change workspace membership, create nested workspaces, override target/build directories, or clean shared caches to work around contention. A busy pool waits; malformed registered members must be fixed in place.
- Keep Macroquad pinned exactly to `=0.4.16`, including feature-bearing dependencies. See `rust_management/docs/CARGO_WORKSPACE.md` for the pool, sccache, editor setup and coordinated dependency upgrades.
- Edit canonical root configuration in `rust_management/workspace/`, install it with `python rust_management/sync-workspace.py`, and record intentional root lock changes with `--capture-lock`.

## Validation

- Keep tests in each crate's `tests/` directory and strongly target five cases per major feature; preserve useful regression coverage (§11).
- After meaningful game changes, run `.\publish.ps1` without parameters in the affected project and report the result or blocker. Do not substitute a local run unless requested (§8.3).
- Run formatting, Clippy, source-size checks, tests, and publishing against the actual project checkout being changed and its real workspace/dependency configuration. Do not create or use an isolated project copy, copied source tree, temporary clone, or alternate manifest to bypass failures. A pass in such a copy is not validation of the actual project; report the original failure as a blocker instead (§8.3).
- Store screenshots directly in `docs/verification/`, replacing captures of the same screen or state (§12).
- For UI changes, complete the `UI_STYLE.md` visual review at normal and minimum supported sizes, including relevant dense states and touch interactions; report evidence and limitations.

## Workspace hygiene

- Do not create disposable review files, scratch projects, backup captures, or temporary cleanup directories anywhere, including OS temp directories. Moving them outside the repository is not a workaround. Use the existing tools and keep only required, durable verification evidence at its documented path (§12).
- Capture directly to stable filenames in `docs/verification/` and replace the same screen/state in place. Do not create `baseline_tmp`, `review_*`, `review_cleanup_*`, or `.previous` copies. Read command output directly instead of writing ad hoc logs.
- Never move files out of a repository to satisfy a clean Git status or evade staging rules. Preserve existing work in place; report a blocker if it cannot be handled within the task.
- Never add dummy `Cargo.toml`, `lib.rs`, placeholder crates, or fabricated source files to make workspace checks pass. Workspace membership is explicit in `rust_management/workspace/Cargo.toml`. Inspect and report the offending path; do not modify another project's files or the workspace membership to conceal the failure.
- If an unexpected directory blocks validation, determine its ownership and contents before acting. Remove only verified disposable artifacts created by the current task, within authorized scope; otherwise report the blocker. Do not repeatedly create/delete placeholder files or claim validation passed against a fabricated workspace.
- Standard build outputs and internal files managed and cleaned by established tools are distinct from agent-created scratch files. Use the shared capture wrapper, keep its hidden-window default, wait for completion, and verify its launched game exits. If it fails or leaves a process running, report or fix the tool rather than inventing a temporary project or alternate capture pipeline.

## Commits

- Follow `rust_management/docs/COMMIT_STYLE.md` (relative to the workspace root): a subject in the game's voice ending with a clear parenthetical tag, an honest explanatory body, and AI co-authorship. No Conventional-Commits prefixes or forced metaphors for mechanical changes.
- Read `mytherra` or `stellar_legacy` history before the first commit in a new game.
- Finish, validate, and commit each independently useful major change before starting the next. Keep exploratory edits uncommitted until their outcome is known.
- After implementation and validation, stage and commit all modified and untracked project files, including pre-existing changes, unless the user asks otherwise.
- Never cherry-pick a subset of touched files or hunks to commit, or leave changes uncommitted when finishing. Preserve existing work; do not discard changes just to make the working tree clean.
- Work directly on `master`; do not create a branch unless the user explicitly requests one.
- Before finishing, verify that `git status --short` is empty and report the commit hash and validation results. If a blocker prevents committing, report it explicitly rather than claiming the work is complete.
