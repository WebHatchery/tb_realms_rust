# RustGames Agent Instructions

These instructions apply to all Rust game projects in this workspace.

## Project Standards

- Build games with Rust, `macroquad`, and the shared `macroquad-toolkit` by default.
- Treat missing runtime, rendering, input, asset, or platform behavior as potential `macroquad-toolkit` upgrades before creating project-local alternatives.
- Route JSON game-data parsing and file loading through `macroquad_toolkit::data_loader`; projects own their typed schemas and game-specific validation, while the toolkit owns embedded/runtime loading, platform differences, source-labeled errors, and fallback behavior. Do not add project-local generic JSON loader wrappers.
- Only diverge from the shared toolkit when an existing project has a clear, established alternative or the need is genuinely game-specific.
- Keep every `.rs` file at or below 800 total lines, with no exceptions or excluded sections. Split large files by responsibility before they become difficult to scan or test.
- Prefer small modules with explicit ownership of input, update logic, rendering, assets, and game state.
- Use Rust's named module source filenames (`foo.rs`, `foo/bar.rs`) instead of `foo/mod.rs`. Do not create new `mod.rs` files.
- Keep gameplay logic deterministic where practical. Isolate randomness behind small helper functions or state-owned RNG.
- Avoid broad refactors while making focused changes. Match the style, naming, and structure already present in each project.
- Use clear error handling for asset loading, save/load, publishing, and platform integration.
- Do not introduce new dependencies unless they remove real complexity or match an established project pattern.
- Keep a root-level `catalog_thumbnail.png` for the WebHatchery games catalog. It should be a title-screen capture when available; `publish.ps1` deploys it as `<game_slug>/catalog_thumbnail.png`.

## Macroquad Conventions

- Use `macroquad` for the runtime loop, input, drawing, textures, audio, and timing.
- Ship browser games touch-first. A player must be able to start a new game and complete every required tutorial, core interaction, and recovery action using visible tap/click targets alone; a physical keyboard is never required.
- Keyboard shortcuts may supplement touch controls, but they must not be the only path to an action. Do not show keyboard-command strings in player-facing HUDs, prompts, menus, notices, or tutorials unless the same text also names the visible touch control that performs the action.
- Tutorial prompts must state the exact visible control or direct touch gesture needed next (for example, “Tap CONTINUE” or “Drag the map”). Never ask players to “dismiss,” “confirm,” or perform another action without a tappable target or an explicit touch instruction.
- Keep drawing code separate from state mutation where possible.
- Treat screen size, scaling, and camera transforms as first-class concerns. Games should remain playable at common desktop browser sizes.
- Avoid hard-coded absolute positions unless they are intentionally tied to a fixed virtual resolution.
- Load assets through project-local asset paths and keep missing asset behavior obvious during publishing.

## Testing And Validation

- Store unit tests in separate child files, never inline in implementation files. Use `#[cfg(test)] mod tests;` in `foo.rs` with the tests in `foo/tests.rs` so `use super::*` and private-item access continue to work. See `CODE_STANDARDS.md` §11.3.
- Keep every test `.rs` file at or below 800 total lines. Split larger test suites into focused child modules before they reach the limit.
- Use each project's `publish.ps1` script as the validation path.
- Do not treat running a local instance or local dev server as the required test path unless the user explicitly asks for it.
- After meaningful changes, run `.\publish.ps1` with no parameters from the affected project directory and report whether it passes.
- If `publish.ps1` is missing, blocked, or fails for an unrelated environment reason, report that clearly instead of substituting an unrequested local run.
- Store verification screenshots directly in `docs/verification/`, with no subfolders. When a capture represents the same screen or state as an existing image, replace that image instead of adding a duplicate.

## Commit Messages

- Follow the catalog's commit convention, documented in `rust_management/docs/COMMIT_STYLE.md` (relative to the workspace root). It is not copied into game projects — read it there.
- The shape: the subject narrates the change in the game's own voice and ends with a plain-terms parenthetical tag (subsystem, GDD section, and/or milestone); the body is honest prose covering problem, change, and reasoning.
- Copy the shape, not another game's metaphors. Each game speaks in its own fiction, and the same technical concept should map to the same fictional term in every commit for that game.
- A reader who ignores the metaphor and reads only the parenthetical must still know exactly what the commit does. Do not omit the parenthetical, and do not force a metaphor onto a trivial mechanical change.
- No Conventional-Commits prefixes (`feat:`, `fix:`, `chore:`, `refactor:`).
- `mytherra` and `stellar_legacy` are the worked exemplars; read either project's `git log` before your first commit in a new game.
- After completing a requested implementation and its required validation, check the working tree and commit the finished changes unless the user explicitly asks to leave them uncommitted. Report the commit hash and validation result in the handoff.
- When a request contains multiple independently useful changes, finish, validate, and commit each major change before beginning the next one. Keep exploratory edits uncommitted until their outcome is known, but do not combine unrelated fixes, UI polish, or refactors into one commit merely because they occurred in the same task.
- Before each requested commit, stage every modified and untracked project file, including pre-existing changes not created during the current task. Do not leave local project changes uncommitted.

## File Size Rule

- Keep every `.rs` file at or below 800 total lines. The limit applies without exception to implementation files, test files, generated Rust source, examples, build scripts, and benches.
- Count every physical line, including tests, comments, attributes, and whitespace.
- Treat a file reaching or approaching 800 lines as a restructure signal, not as a formatting target.
- Do not preserve the limit by stripping useful spacing, compressing formatting, moving a single small function, or making other cosmetic line-count changes.
- If a meaningful change would push a file over the limit, extract a cohesive responsibility into one or more nearby modules before or alongside the change.
- If any file is already over 800 lines, make the restructure part of the current task before considering the task complete.
