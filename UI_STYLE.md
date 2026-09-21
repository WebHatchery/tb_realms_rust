# Game UI Style

Applies to new games and changes to existing game screens. This is the shared
authority for screen composition, information hierarchy, and visual review.
[CODE_STANDARDS.md](CODE_STANDARDS.md) §7 governs UI code and touch controls;
[MACROQUAD_TOOLKIT.md](MACROQUAD_TOOLKIT.md) supplies implementation tools;
[GAME_DEVELOPMENT_GUIDE.md](GAME_DEVELOPMENT_GUIDE.md) covers setup and workflow.
Edit the canonical `rust_management/docs/UI_STYLE.md` and distribute it with
`sync-project-docs.ps1`. Keep each game's screen plans in its GDD or README.

> Show the player the world, the current problem, and the actions that matter
> now. Hide everything else until it earns its place.

## 1. Plan the player's decision before the widgets

Before building or substantially changing a screen, record this short brief
in the game's GDD or README. Update it when the gameplay changes.

| Question | Required answer |
| --- | --- |
| Current decision | What is the player trying to decide or do in this phase? |
| Dominant focus | Which world view, encounter, object, or comparison should draw the eye first? |
| Primary action | What action advances play, and which cost, risk, or constraint belongs beside it? |
| Supporting information | What must remain visible to make that decision? |
| Deferred information | What appears on selection, in help, in another phase, or after discovery? |
| Layout and camera | What fills the play area at the normal and smallest supported viewport? |
| Input and feedback | How does a touch player discover, perform, and recognize the result of the action? |

Design from the player's current understanding of the game. A simulation
system or toolkit widget does not automatically deserve a permanent panel.

## 2. Give each screen one dominant purpose

- **Attention budget:** No more than 2–3 regions should demand strong attention
  during normal play.
- Aim for one dominant focus, one supporting area, and quiet utilities. Use
  emphasis, placement, and scale to make the first place to look clear within
  about a second. These are attention roles, not a mandatory three-column grid.
- Give the main play area most of the screen during normal play. Show the
  dungeon, colony, ship, or auction at a useful scale; do not shrink it to make
  room for a permanent dashboard. In a comparison or planning phase, the
  comparison itself can be the main play area.
- Prefer a few purposeful regions to many equal cards. Keep costs beside the
  action they constrain, warnings beside the affected decision, and character
  states beside that character.
- Usually give the action that advances play the strongest treatment. Quiet
  utilities such as settings or save management should not outrank it. Equally
  valid choices can share emphasis; do not invent a preferred choice.
- **Gameplay vs navigation:** Never visually group gameplay decisions with
  menu, save, exit, or settings actions.

For an auction, the current lot and bid are dominant, rivals support the
decision, and affordable limits sit beside the bid control. Detailed research
belongs in preparation or inspection. A save button should not be the focal
point of the auction.

## 3. Use hierarchy before adding boxes or shrinking content

- Distinguish primary, secondary, and tertiary information with size, contrast,
  position, and spacing. Keep even quiet information readable. Hide irrelevant
  information instead of making everything equally tiny.
- Use borders only to communicate meaningful grouping, interaction, selection,
  or separation. Prefer spacing, alignment, typography, or background tone
  when those already communicate the relationship. Avoid nested panels and
  a labelled container around every fact.
- Labels, tabs, filters, badges, and cards must solve a player need. Remove
  headings that repeat what the image, position, or interaction already says.
  If a gameplay screen resembles an admin dashboard, revisit its current
  decision and remove unrelated systems. A deliberate management table is
  appropriate when comparing its entries is the actual gameplay decision.
- Use a coherent type scale, spacing rhythm, and semantic colors appropriate
  to the game. Reserve strong emphasis for actions and meaningful changes;
  do not give every surface the same visual weight.
- Use empty space to frame the focus and separate unrelated things. After
  removing panels, recompose and enlarge the relevant content. Scattering the
  remaining controls into corners leaves an empty screen, not a focused one.

## 4. Make permanent information earn its place

For each element, ask whether the player needs it throughout this phase.
If not, show it on selection, in a dismissible inspector, through a visible
disclosure control, briefly after an event, or on a separate screen.

- Keep normal play focused on the current phase. Research, trading, fighting,
  and exploration need different information; avoid carrying every system
  between them.
- Reveal advanced systems and information as they become relevant or are
  learned through play. Unknown facts can remain unknown until researched;
  do not expose the full simulation at the start.
- Preserve the costs, consequences, constraints, and urgent warnings needed
  to make an informed action. Progressive disclosure must not hide actionable
  information merely to make a screenshot cleaner.
- Give each fact one clear home. Avoid repeating a leader in a banner, log,
  badge, and character panel at the same time. Temporary change feedback may
  accompany the persistent current value, then disappear.
- Keep save management, settings, debug statistics, implementation details,
  and long explanations out of the normal gameplay focus. Preserve a visible,
  understandable route to utilities and recovery controls.

## 5. Teach briefly and keep controls discoverable

- Use onboarding, first-use prompts, and contextual help to teach mechanics.
  Prompts name the exact visible control or gesture, as required by
  `CODE_STANDARDS.md` §7.5. Dismiss completed instructions and provide a visible
  way to reopen help.
- Remove repeated tutorial prose from ordinary play. Keep meaningful action
  labels and unfamiliar symbols understandable without the tutorial.
- Hover can supplement inspection but must have a tap/click equivalent.
  Required actions, dismissal, navigation, and recovery must work without a
  keyboard, hover, or right mouse button.
- Contextual controls must be visibly discoverable when needed. Do not trade
  clutter for hidden gestures, unexplained icons, or missing touch controls.
- Disabled actions should communicate their reason where the player needs
  it. Place the relevant shortage or prerequisite near the action.

## 6. Let the world and its changes communicate

- **State vs event:** Persistent UI shows current state; temporary feedback
  shows what just changed.
- Prefer readable changes in the world: a raised bidding paddle, damaged
  machinery, smoke, changing posture, or rising water. Supplement these with
  UI when the player needs an exact value or an otherwise ambiguous signal.
- Acknowledge actions with changes in the affected object, short animation,
  or temporary feedback. Avoid permanent status prose describing every state.
- Keep calm phases restrained. As pressure rises, focus attention on the
  immediate consequences and suppress unrelated information. Avoid turning
  every event into an equally urgent alert.
- Important results must remain understandable after an animation or toast
  ends, through the updated state or a retrievable history. Do not rely only
  on color, motion, sound, or a fleeting message to communicate a critical fact.

## 7. Compose for the actual viewport and camera

- Declare normal and minimum supported viewport sizes in the game README or
  GDD. Check the actual browser canvas, including embedded play, as well as
  native rendering. A virtual resolution alone does not prove usability.
- Set camera framing and default zoom around what the player must perceive
  and select. UI should wrap around the play area without obscuring targets
  or squeezing the world into a small leftover rectangle.
- At smaller sizes, prioritize, reflow, collapse, or move secondary content
  before reducing text and controls. Keep the primary action reachable;
  contain long collections with deliberate scrolling or separate views.
- Test long labels, large values, dense content, and expanded inspectors.
  Text fitting is an overflow safeguard, not permission to render tiny text.
  Recheck tap targets and spacing after virtual scaling.
- Reuse toolkit layout, camera, and pointer helpers so rendering and input
  agree. Keep layout and pointer math in logical coordinates and use the
  toolkit's viewport conversion at the framebuffer boundary. Verify picking
  after resize, zoom, and display scaling changes.

## 8. Treat the template as integration examples

`rust_management/template/` demonstrates toolkit wiring. Its demo panels,
action cards, permanent help, technical labels, and save/debug controls are
examples to adapt; they are not an approved final layout for every new game.

After copying the template, write the screen brief and recompose the first
playable screen around the game's decision before expanding its content.
Remove demo copy and unused surfaces, relocate utilities, set a useful camera
scale, and establish the primary action. Retain useful input, state, loading,
persistence, and capture patterns. Choose toolkit widgets after deciding what
the player needs to see; reusing a widget does not require displaying it.

## 9. Review by subtraction, then verify in play

For an existing screen, first remove duplicate facts, dead headings, permanent
help, unused panels, decorative borders, and irrelevant statistics. Recompose
the remaining content and adjust the camera. Add elements only when a player
decision or observed usability gap calls for them.

Before accepting a new or changed UI, inspect screenshots and exercise the
affected interactions. Cover normal play at the normal and minimum supported
sizes, plus relevant first-use, dense/late-game, selected/expanded, and urgent
or failure states. Capture only states the game actually supports.

- [ ] The current decision, focal element, and primary action are clear at a glance.
- [ ] The world or current decision occupies the main area at a useful scale.
- [ ] Each persistent fact helps this phase, has one clear home, and sits near what it affects.
- [ ] Borders, labels, secondary systems, and utilities do not compete with play.
- [ ] Early play reveals only relevant or earned complexity; action costs and risks remain clear.
- [ ] Help is available but completed instructions do not remain as wallpaper.
- [ ] Required interactions and feedback work with touch alone, including inspection and dismissal.
- [ ] Text and targets remain readable and usable, with no clipping, overlap, or obscured actions.
- [ ] Feedback remains understandable after temporary effects end.
- [ ] Removing clutter improves composition rather than leaving accidental empty space.

Store captures directly in `docs/verification/`, replacing equivalent existing
captures, per `CODE_STANDARDS.md` §12. Use the game's capture harness where
available and verify browser/touch behavior interactively. Report the scenes,
sizes, interactions checked, and any limitations. A successful compile or a
screenshot with no overlap does not establish a usable visual hierarchy.
Follow the affected game's normal publish validation under §8.3 as well.
