# RustGames Agent Instructions

Applies to RustGames projects. Edit shared guidance in `rust_management/docs/`;
keep project rules in README/GDD or `PROJECT_AGENTS.md`. Read those local rules
and inspect Git status before editing. User instructions take precedence.
For prototype validation and slice commits, this policy supersedes older generic
full-suite/publish/all-screen-review or blanket save-migration mandates repeated
in project notes or skills. Preserve project-specific design and explicit current
user exceptions; backward save compatibility is optional unless requested.

## Read for the task

Read the indicated authority **before** the relevant work; references are
required when triggered, not a request to load every document each turn.
Paths below are beside this file, both in canonical docs and synced projects.
Management-only references use workspace-root paths in code spans.

| Task | Required reference |
| --- | --- |
| Change Rust, data, assets, or behavior | [CODE_STANDARDS.md](CODE_STANDARDS.md) |
| Design/change screens, controls, camera, or rendering | [UI_STYLE.md](UI_STYLE.md) and relevant [toolkit modules](MACROQUAD_TOOLKIT.md#modules) |
| Add/change a shared runtime, input, asset, platform, or UI capability | Relevant [MACROQUAD_TOOLKIT.md](MACROQUAD_TOOLKIT.md) sections; consider a toolkit upgrade before a local alternative |
| Add, change, or delete tests | [Testing policy](CODE_STANDARDS.md#11-testing-policy) |
| Validate or capture; encounter workspace failures | [Validation](CODE_STANDARDS.md#83-validation) and [artifact hygiene](CODE_STANDARDS.md#12-verification-artifacts) |
| Create/migrate a game | [GAME_DEVELOPMENT_GUIDE.md](GAME_DEVELOPMENT_GUIDE.md) and the template README rename steps |
| Change build/workspace/dependency configuration | `rust_management/docs/CARGO_WORKSPACE.md` |
| Commit | Commits below; `rust_management/docs/COMMIT_STYLE.md` for message examples |

## Always enforce

- Use Rust, Macroquad, and `macroquad-toolkit`; diverge only for an established
  project pattern or genuinely game-specific need. Match style, avoid unrelated
  refactors, and add dependencies only when they remove real complexity or match
  an established pattern. Keep gameplay deterministic where practical.
- Every `.rs` file has an **800-total-line hard limit, no exemptions**. Extract
  cohesive modules; never compress formatting to pass. No new `mod.rs`.
- UI reads state and returns intents; game logic owns mutations. Load JSON
  content through the toolkit; projects own schemas and semantic validation.
- Browser play must work through visible touch controls, including tutorials
  and recovery. Plan the current decision and dominant play area; simplify
  existing screens and recompose template demos. Review only affected screens,
  states, sizes, and touch interactions; compilation alone is not visual review.
- Use `..\rust_management\cargo.ps1` for build/check/test/Clippy/run; formatting
  may use Cargo directly. Publish/capture use the same three-slot pool. Quote
  PowerShell's separator: `cargo.ps1 clippy '--' -D warnings`.
- Pin Macroquad exactly to `=0.4.16`, including feature-bearing dependencies.
  Edit root configuration in `rust_management/workspace/`, install with
  `sync-workspace.py`, and capture intentional root lock changes with `--capture-lock`.
- Validate the actual checkout and real dependencies. Never bypass failures via
  copied projects, alternate manifests, fabricated crates, workspace membership
  changes, nested workspaces, target overrides, or cache cleanup. A busy pool waits.
- Preserve existing work in place. No disposable review/scratch files, backup
  captures, or cleanup folders anywhere, including OS temp. Only remove verified
  disposable artifacts created by this task and within scope; report other blockers.
  Established tools' managed internal files and normal build outputs are allowed.
- Capture directly to stable filenames in `docs/verification/`, no subfolders
  or duplicates. Use the shared wrapper's hidden default, wait, and verify game
  exit; fix/report tool failures instead of inventing alternate capture pipelines.
- Validate slices with formatting, strict Clippy, source-size gates, and focused
  existing tests. Broader suites need a cross-cutting/integration/release reason;
  publishing never gates a local commit and requires user authorization. Follow
  the validation reference for scope, unchanged results, and failures.
- Projects are prototypes: **do not write tests by default**, even once
  released. Add one only for a settled complex algorithm, data integrity, or a
  recurring regression; never for UI layout, experimental rules, temporary
  balancing, or implementation details. When changing experimental behaviour, delete the tests that asserted
  it rather than rewriting them. Keep the source-size and asset-registry gates.
  Report what was and was not verified. See [Testing](CODE_STANDARDS.md#11-testing-policy).
- Demo saves may break compatibility: reject unsupported old saves with a clear
  recoverable error, never a crash. Migrations are optional unless requested.

## Commits

- **Review, validate, and commit each small, coherent, independently useful
  feature slice before starting the next.** Do not wait for several major
  features or a milestone. Keep each slice buildable; never commit broken or
  exploratory code merely for cadence. Use proportionate
  [validation](CODE_STANDARDS.md#83-validation), not a full suite/publish per slice.
- Review status, full and staged diffs. Commit all required code, tests, data,
  docs, and artifacts for the slice. Preserve pre-existing/user/concurrent work;
  include it only with clear ownership/authorization and when it belongs to the
  slice. Report inseparable overlaps instead of overwriting or staging another
  task. Never omit required files just to obtain a passing commit.
- Work on `master` unless the user requests a branch. Committing authorizes no push or external publication.
- Use a present-tense subject in the game's voice with a clear `(technical tag)`,
  an honest explanatory body with verification, and AI co-authorship. No
  Conventional-Commits prefixes or forced metaphors for mechanical changes.
  Read `mytherra` or `stellar_legacy` history before the first commit in a new game.
- Finish with no task-owned changes uncommitted. Check `git status --short` and
  report hashes, checks, preserved unrelated changes, and blockers honestly.
