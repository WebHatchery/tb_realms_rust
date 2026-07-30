# RustGames Agent Instructions

These instructions apply to all Rust game projects in this workspace.

## Project Standards

- Build games with Rust, `macroquad`, and the shared `macroquad-toolkit` by default.
- Treat missing runtime, rendering, input, asset, or platform behavior as potential `macroquad-toolkit` upgrades before creating project-local alternatives.
- Only diverge from the shared toolkit when an existing project has a clear, established alternative or the need is genuinely game-specific.
- Keep source files under 800 lines, counting non-test lines only. Split large files by responsibility before they become difficult to scan or test.
- Prefer small modules with explicit ownership of input, update logic, rendering, assets, and game state.
- Use Rust's named module source filenames (`foo.rs`, `foo/bar.rs`) instead of `foo/mod.rs`. Do not create new `mod.rs` files.
- Keep gameplay logic deterministic where practical. Isolate randomness behind small helper functions or state-owned RNG.
- Avoid broad refactors while making focused changes. Match the style, naming, and structure already present in each project.
- Use clear error handling for asset loading, save/load, publishing, and platform integration.
- Do not introduce new dependencies unless they remove real complexity or match an established project pattern.
- Keep a root-level `catalog_thumbnail.png` for the WebHatchery games catalog. It should be a title-screen capture when available; `publish.ps1` deploys it as `<game_slug>/catalog_thumbnail.png`.

## Macroquad Conventions

- Use `macroquad` for the runtime loop, input, drawing, textures, audio, and timing.
- Keep drawing code separate from state mutation where possible.
- Treat screen size, scaling, and camera transforms as first-class concerns. Games should remain playable at common desktop browser sizes.
- Avoid hard-coded absolute positions unless they are intentionally tied to a fixed virtual resolution.
- Load assets through project-local asset paths and keep missing asset behavior obvious during publishing.

## Testing And Validation

- Keep unit tests in the crate, as an inline `#[cfg(test)] mod tests` block at the bottom of the file they cover. Do not move them to a `tests/` directory — those are separate integration crates limited to the public API, and most games here are binary crates that `tests/` cannot import at all.
- When a test module exceeds ~300 lines or more than half its file, extract it to a child module (`#[cfg(test)] mod tests;` in `foo.rs` -> `foo/tests.rs`), which keeps `use super::*` access. See `CODE_STANDARDS.md` §11.3.
- Use each project's `publish.ps1` script as the validation path.
- Do not treat running a local instance or local dev server as the required test path unless the user explicitly asks for it.
- After meaningful changes, run `.\publish.ps1` with no parameters from the affected project directory and report whether it passes.
- If `publish.ps1` is missing, blocked, or fails for an unrelated environment reason, report that clearly instead of substituting an unrequested local run.

## Commit Messages

- Follow the catalog's commit convention, documented in `rust_management/docs/COMMIT_STYLE.md` (relative to the workspace root). It is not copied into game projects — read it there.
- The shape: the subject narrates the change in the game's own voice and ends with a plain-terms parenthetical tag (subsystem, GDD section, and/or milestone); the body is honest prose covering problem, change, and reasoning.
- Copy the shape, not another game's metaphors. Each game speaks in its own fiction, and the same technical concept should map to the same fictional term in every commit for that game.
- A reader who ignores the metaphor and reads only the parenthetical must still know exactly what the commit does. Do not omit the parenthetical, and do not force a metaphor onto a trivial mechanical change.
- No Conventional-Commits prefixes (`feat:`, `fix:`, `chore:`, `refactor:`).
- `mytherra` and `stellar_legacy` are the worked exemplars; read either project's `git log` before your first commit in a new game.

## File Size Rule

- Keep every `.rs` file below 800 lines.
- Count non-test lines only. A `#[cfg(test)] mod tests` block does not count toward the limit — it is one cohesive block at the bottom of the file and is compiled out of release and WASM builds entirely.
- Treat a file reaching or approaching 800 lines as a restructure signal, not as a formatting target.
- Do not preserve the limit by stripping useful spacing, compressing formatting, moving a single small function, or making other cosmetic line-count changes.
- If a meaningful change would push a file over the limit, extract a cohesive responsibility into one or more nearby modules before or alongside the change.
- If a touched file is already over 800 lines, make the restructure part of the current task, or queue it as the next work item before considering the task complete.
