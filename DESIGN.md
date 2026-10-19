---
name: SQLKit
description: A restrained, dense, cross-platform database workspace centered on SQL and data
colors:
  primary-light: "hsl(221.2 83.2% 45%)"
  primary-dark: "hsl(217.2 91.2% 45%)"
  background-light: "hsl(0 0% 100%)"
  background-dark: "hsl(222.2 84% 4.9%)"
  foreground-light: "hsl(222.2 84% 4.9%)"
  foreground-dark: "hsl(210 40% 98%)"
  card-light: "hsl(0 0% 100%)"
  card-dark: "hsl(222.2 47% 8%)"
rounded:
  lg: "8px"
  md: "6px"
  sm: "4px"
---

# SQLKit Design Specification

## Overview

**Design direction: A quiet SQL workspace.** Users should notice connection context, SQL, and query results first, actions second, and brand expression last. Support long sessions of writing, inspecting, and comparing data through high information density, clear hierarchy, stable layouts, and predictable interactions.

This file defines the design constraints for new and modified SQLKit client UI, alongside [AGENTS.md](AGENTS.md). Preserve the existing blue theme, light / dark / system modes, compact desktop layout, and shadcn-vue / Radix Vue component system. Establishing this specification does not mean existing screens have already been migrated.

### Sources and adaptations

The reference project is `console-serverlessinsight`, reviewed on 2026-10-19. The main sources are its `AGENTS.md` section on color rules (V3 · Mono + Violet Accent), `apps/console/src/assets/css/themes/violet.css`, `apps/console/uno.config.ts`, `docs/console-ui-design.md`, and `apps/console/PRODUCT.md`. These paths record provenance only; SQLKit has no runtime dependency on the reference repository.

| Reference principle | SQLKit adaptation |
| --- | --- |
| Give colors fixed semantics rather than assigning them to modules | Blue for primary actions and interaction, green for success, red for errors or danger; neutrals for ordinary information |
| Let neutral surfaces dominate and keep accents scarce | Preserve SQLKit's slightly cool neutral tokens rather than copying the reference project's zero-chroma grays |
| Restrict brand color to small areas | Keep SQLKit's blue primary buttons, limiting their count and area; do not copy violet or monochrome primary buttons |
| Use semantic status tokens across light and dark themes | Reuse success / destructive; fill gaps such as warning through the process below |
| Use progressive disclosure, a stable app shell, and keyboard access | Apply these to the connection tree, query tabs, editor, result grids, and desktop window behavior |
| Require non-color signals and readable contrast | Treat these as acceptance criteria; existing tokens do not prove compliance |

The reference project's early colorful diagrams, login gradients, and CSS glass and ambient-light effects are not design references for SQLKit's workspace.

### Specification and implementation

- **Current baseline:** Values explicitly tied to code below describe the current implementation. The sole runtime source of color values is `src/assets/index.css`; utility mappings live in `uno.config.ts`.
- **Design constraints:** “Must,” “must not,” and “default” apply to new and modified UI. Historical implementations do not justify extending existing problems.
- **Migration work:** Tokens, component cleanup, and interactions that remain unimplemented are listed at the end. Do not treat documented targets as existing APIs.
- This file governs system-wide design decisions; feature documents describe behavior. When changing theme colors, shared component styles, or density baselines, update this file and the implementation together.

## Colors

### Current theme baseline

The table records CSS HSL components, used as `hsl(var(--token))`. The frontmatter contains a subset of implemented tokens for tooling. CSS defines the complete theme; keep the documentation synchronized and never copy these values into components.

| Token | Light | Dark | Role |
| --- | --- | --- | --- |
| `--primary` | `221.2 83.2% 45%` | `217.2 91.2% 45%` | Primary actions, links, interaction emphasis |
| `--primary-foreground` | `0 0% 100%` | `0 0% 100%` | Text on solid primary backgrounds |
| `--background` | `0 0% 100%` | `222.2 84% 4.9%` | Main workspace |
| `--foreground` | `222.2 84% 4.9%` | `210 40% 98%` | Body text and important data |
| `--card` / `--popover` | `0 0% 100%` | `222.2 47% 8%` | Containers and overlays |
| `--secondary` / `--muted` | `210 40% 96.1%` | `217.2 32.6% 16%` | Secondary surfaces and neutral states |
| `--muted-foreground` | `215.4 16.3% 46.9%` | `215 20.2% 65.1%` | Supporting text; never the sole carrier of critical information |
| `--accent` | `210 40% 96.1%` | `217.2 32.6% 17.5%` | Neutral interaction surface, **not a second brand color** |
| `--border` | `214.3 31.8% 91.4%` | `217.2 32.6% 17.5%` | Structural separation |
| `--input` | `214.3 31.8% 80%` | `217.2 32.6% 28%` | Input boundaries |
| `--ring` | `220 13% 60%` | `212.7 26.8% 68%` | Keyboard focus; retain the current neutral color |
| `--success` | `142.1 70.6% 35.5%` | `142.1 70% 45%` | Success, connected, completed |
| `--success-muted` | `138 76% 95%` | `142 35% 16%` | Local success feedback background |
| `--success-border` | `140 58% 85%` | `142 30% 25%` | Success feedback boundary |
| `--destructive` | `0 84.2% 60.2%` | `0 62.8% 30.6%` | Errors, failures, destructive actions |

### Semantic ownership

| Visual signal | Allowed meaning | Must not represent |
| --- | --- | --- |
| Blue `primary` | Primary actions, links, selected controls, small active indicators | Success, ordinary badges, large panel backgrounds |
| Neutrals `foreground / muted / secondary / accent` | Body text, supporting information, toolbars, navigation backgrounds, counts, ordinary attributes, disconnected states | Important errors or risks hidden through low contrast |
| Green `success` | Successful connection tests, connected states, successful execution, completed transfers | Execute buttons, boolean true, auto-increment attributes, decorative selling points |
| Red `destructive` | Execution failures, connection failures, deletion confirmations | Ordinary task counts, disconnected states, boolean false, decoration |
| Amber `warning` (not yet implemented) | Actual warnings, risks, exceptional states requiring attention | JSON format, primary keys, ordinary unsaved indicators, arbitrary toolbar buttons |

Use neutral icons and text for ordinary information; do not introduce a separate sky-blue palette. SQLKit's `primary` already owns blue interaction emphasis. There are currently no `--info` or `--warning` mappings. Add tokens only for semantics that require a distinct role.

### Color budget

1. **Start with neutrals, then add emphasis.** Most backgrounds, borders, body text, and icons in a resting workspace use existing neutral tokens. Establish hierarchy through typography, spacing, and position first.
2. **Aim for accents to occupy no more than roughly 10% of the UI.** This is a review heuristic for the app shell, toolbars, panels, and forms, not a pixel-counting requirement. Evaluate SQL syntax highlighting, actual data charts, and local error feedback separately for readability; these exceptions must not expand decorative color use.
3. **Use at most one solid primary action per operation context.** This is usually Execute in a query workspace or Save in a form dialog. Other actions use outline / secondary / ghost. An independent dialog may have its own primary action. Focus rings and status dots do not count as a second solid primary action.
4. **Success, warning, and failure may coexist when backed by actual state.** Default to an icon / status dot with text, adding a local, subdued background when needed. Do not tint an entire row, card, or panel.
5. **Features do not own colors.** Do not give each of CSV / JSON / INSERT, copy / export / format / EXPLAIN, or individual filter operators a separate color.

### Allowed and avoided treatments

| Context | Default treatment | Avoid |
| --- | --- | --- |
| Toolbars | Neutral icons and text; `accent` on hover | Blue / purple / orange / green buttons side by side |
| Navigation / tabs | Neutral backgrounds and font weight; small blue indicators when needed | Solid saturated blue blocks or multicolor navigation entries |
| Attribute badges | Explicitly select `outline` or `secondary` on `Badge` | Solid blue badges chosen merely because that variant is the default |
| Connection cards | Name, engine icon, status text / dot | A different colored background for each database engine |
| Alerts / notifications | Neutral surfaces, status icons, readable text | Large saturated red or green backgrounds |
| AI assistant / Ultimate indicators | The same neutral containers and small primary accents | A separate purple theme, rainbow gradients, glowing borders |

### Color exceptions

- Monaco SQL syntax highlighting, code diffs, and actual data charts may use multiple colors when those colors explain content. Do not extend their palettes into application controls.
- Existing database vendor logos may retain identifying colors inside small icons. Do not extend those colors to cards, buttons, or navigation backgrounds.
- Charts should emphasize the series of interest with primary and use neutrals for the others. When multiple series need distinction, a dedicated chart palette is allowed with labels, legends, or line styles. `--chart-1` through `--chart-5` exist in CSS but are not registered as UnoCSS color names; do not assume `text-chart-1` works.
- Data diffs may use success / destructive for additions and deletions, accompanied by `+ / −` or text. Ordinary boolean values do not represent success or failure.

### Token implementation process

1. Check whether an existing role can express the meaning before considering opacity variants. Opacity does not waive contrast requirements: text must remain readable against the actual composited background.
2. Do not add hex / RGB / HSL literals or palette utilities such as `blue-500`, `green-600`, and `violet-*` in business components to replace existing semantic roles. Isolate chart, syntax, and vendor-asset exceptions in their respective modules.
3. When adding a semantic token, include all of the following in the same change: `:root` and `.dark` definitions in `src/assets/index.css`, necessary foreground / subdued-background pairs, `uno.config.ts` mappings, shared component variants where needed, light/dark verification, and this document's update.
4. Do not submit UI that depends on unimplemented utilities such as `text-warning` or `bg-info` before their mappings exist. Warning is a known gap; do not substitute `primary` or `success` for it.
5. Check all consumers before changing shared tokens. Fixing semantic color readability must not also replace the blue brand or reset the entire neutral palette.

## Typography

### Fonts and hierarchy

The global body size is currently 14px, and `src/assets/index.css` does not define a custom brand font. Keep system sans-serif fonts with platform Chinese fallbacks. Do not introduce decorative fonts or remote font dependencies for the client. Keep Monaco's monospace editor font; SQL, types, object identifiers, and code snippets may use the existing `font-mono` utility.

The following hierarchy constrains new UI; it does not require rewriting all existing page titles.

| Role | Size / weight | Usage |
| --- | --- | --- |
| Page title | 20–24px / 600 | Standalone pages such as Connections and Settings; do not add large headings to editor workspaces |
| Dialog / section title | 14–18px / 500–600 | Dialog titles and major groups |
| Body / form | 14px / 400 | Descriptions, inputs, buttons; matches the current `text-sm` |
| Dense data / supporting labels | 12px / 400–500 | Result grids, object trees, toolbars, status bars |

- Use roughly 1.4–1.6 line height for ordinary text. Code follows editor settings; fixed-height table rows must not clip text.
- Do not use 9–10px text for states, prices, field meanings, or instructions users must read. Adjust the layout first when space is tight.
- Prefer `foreground` for names and data, and `muted-foreground` for supporting descriptions. Do not place important information in tiny light-gray text with stacked opacity reductions.
- Use `tabular-nums` for row counts, duration, amounts, and progress. Right-align numeric columns, left-align text columns, and align headers with their contents.
- Long connection names, table names, SQL, and file paths may truncate, but users must be able to obtain the full content through expansion, details, or copying. Critical connection context must not depend solely on hover hints.

## Layout

### Desktop workspace structure

Retain the existing app shell: top window bar → left feature navigation → current workspace. The query page contains the object tree / saved queries, query tabs, editor, and results. AI and tasks occupy supporting panels on the right and remain subordinate to the main task.

| Current implementation | Baseline | Source |
| --- | --- | --- |
| Initial window | 1600 × 1000 | `src-tauri/tauri.conf.json`; not a minimum window size |
| Window bar | 40px | `h-10` in `AppHeader.vue` |
| Feature navigation | 64px wide, 40 × 40px entries | `AppSidebar.vue` |
| Query sidebar | Initially 250px, resize range 200–400px | `QueriesPage.vue` |
| Query tabs | 35px high, maximum 200px per tab | `QueryTabs.vue` |
| Result toolbar | 36px high | `QueryResultPanel.vue` |
| Standard buttons / inputs | 32px high | `ui/button`, `ui/input` |
| Virtual result rows | 32px estimated height | `grid/DataGrid.vue`; not a shared height for every table |

### Spacing and containers

- Use a primarily 4px rhythm: 4 / 8 / 12 / 16 / 24 / 32px. Existing compact controls may keep 6px internal padding.
- Typical gaps are 4–8px between icons and text, 4–8px within action groups, 12–16px inside sections, and 24px between major groups.
- Organize content with separators, headings, and whitespace first. Editors and results are continuous work surfaces; do not wrap every toolbar or field in another card.
- Give each scroll region a clear responsibility. Use `min-w-0` / `min-h-0` to prevent flex children from overflowing the layout. Wide tables scroll horizontally within the result region, not across the entire app shell.
- Give remaining space to data. Reveal advanced connection options, SSL/SSH, and filter details on demand rather than filling the initial viewport.

### Window resizing and platform adaptation

The client primarily supports desktop operation; do not apply a marketing site's mobile card-stacking pattern. The following are acceptance scenarios for future layout changes, not claims that CSS breakpoints or minimum window sizes are already configured:

- Check ordinary workflows at 1600 × 1000 and 1280 × 800, and constrained space around 1024 × 768. Collapse supporting panels and secondary labels first; move infrequent actions into overflow menus.
- Keep the current connection / database, primary action, execution cancellation, and critical errors reachable at any window width. Do not silently clip required information with `overflow-hidden`.
- Dialogs must fit the available window. Long forms scroll internally while Save and Cancel remain reachable. Do not solve overflow by shrinking text.
- Reuse `usePlatform` and `useWindowControls`: preserve the macOS traffic-light inset and support Windows / Linux window controls and fullscreen behavior. Interactive controls must stay outside drag regions.
- Shortcut labels must match actual bindings: `⌘ / ⌥` on macOS, `Ctrl / Alt` on Windows / Linux. Do not hardcode one platform's modifiers in labels, tooltips, or help text.

## Elevation & Depth

**Flat by default; elevate overlays.** Divide workspaces with existing surface levels and borders. Reserve pronounced shadows for floating menus, dialogs, and notifications. Retain the existing Card's `shadow-sm`, but do not repeatedly stack cards, shadows, and borders in new nested containers.

- Use `background / card / muted` for content surfaces and `popover` with its paired foreground for overlays. Do not introduce colored glass surfaces.
- The current `.border` uses a 0.5px border with a 1px fallback. Do not invent per-page border systems. Separators must not be the only signal identifying a control or keyboard focus.
- Focus rings communicate interaction rather than decoration. Do not add global glowing outlines, ambient color blobs, colorful gradient backgrounds, or continuous pulsing animations.
- Overlays must cover their owning panels and remain dismissible. Use existing Radix portals and shared overlay components rather than arbitrary z-index increases.
- Motion should communicate hover, expansion, transitions, and task progress. Prefer 120–200ms for new motion; existing 250ms notification transitions may remain. Avoid `transition-all` animating unrelated properties. Respect `prefers-reduced-motion` and retain static status text.

## Shapes

Keep `--radius: 0.5rem`: `lg` is 8px, `md` is 6px, and `sm` is 4px. Use `rounded-md` for ordinary inputs and buttons, `rounded-lg` for cards and dialogs, and component defaults for compact internal elements.

- Use corner radius to convey component hierarchy rather than inventing per-page values. Keep continuous straight edges for data grids, split panes, and editors.
- Limit `rounded-full` to suitable elements such as avatars, small status dots, and existing badges. Do not make every button pill-shaped.
- Toolbar icons are usually 14–16px and main navigation icons 20px; the visible icon is smaller than its hit target. Keep standard action targets at 32px. Compact grid actions should generally be at least 24px with sufficient spacing and keyboard access.
- Reuse the component's existing Carbon / Lucide style. Do not add icon libraries for color or decoration, or mix emoji into an action group.

## Components

### Shared components and states

Prefer `src/components/ui/*`, importing through each module's `index.ts`. Manage shared variants with CVA; business components select roles and states. Keep icons, labels, and interactions consistent for the same action across pages.

Interactive components must cover default, hover, focus-visible, active / selected, and disabled states. Async actions also need loading, success, and error states. Hover signals interactivity, selected signals selection, and focus-visible signals keyboard position; do not collapse all three into the same blue background.

### Buttons and toolbars

- `default`: the single primary action, retaining `bg-primary text-primary-foreground`. Current default height is 32px; large is 36px.
- `outline / secondary`: secondary actions; `ghost`: toolbar and infrequent actions; `link`: explicit text links.
- `destructive`: genuinely destructive actions such as deletion. Ordinary disconnect and stop-query actions do not automatically become solid red buttons because they stop something.
- Loading states prevent duplicate submission and keep readable labels. Provide cancellation for cancellable long tasks rather than disabling the entire panel.
- Icon buttons must have localized accessible names. Tooltips supplement actions and shortcuts; they do not replace accessible names. The icon, label, and tooltip must describe the same action.
- Explain disabled states, such as no connection, no results, or a running task. Reduced opacity alone is insufficient.

### Navigation, query tabs, and context

- Navigation is neutral by default. An active entry may use a neutral background and blue icon without an additional solid blue block. Provide `aria-current` or equivalent semantics.
- Keep query tabs compact. Distinguish the active tab with background / weight, and unsaved changes with a small marker and accessible explanation. Do not tint the whole tab as a warning.
- Close buttons must appear on hover or keyboard focus. Explain the consequences of closing unsaved work; visual simplification must not remove the save path.
- Always make the current connection, database / schema, and execution target identifiable. Supplement production-environment risks with text / icons rather than relying on remembered connection colors.

### Forms, connection settings, and dialogs

- Reuse 32px Input / Select controls and the existing label / helper / error structure. Placeholders do not replace labels.
- Group host, port, authentication, database, and related fields by task. Progressively reveal advanced SSL / SSH settings, retaining the conditional-field behavior in [SSL_UI_DESIGN.md](docs/SSL_UI_DESIGN.md).
- Place validation near the field and explain the cause and remedy. Preserve entered content after failure; handle sensitive values under existing security policies.
- Give dialogs a clear title, a description matching the content, and reachable close / cancel controls. Manage focus, Escape, and focus restoration after closing.
- Destructive confirmations for deleting data, DROP / TRUNCATE, and similar actions must show the target and impact scope. Describe actual capabilities; do not invent recovery or rollback guarantees.

### Badges, notifications, and task states

- Non-interactive attributes default to `Badge variant="outline"` or `secondary`. Status uses a dot / icon with text. The shared Badge still defaults to solid primary; callers must explicitly select an appropriate variant.
- Use success for connected / completed, neutrals for disconnected / not started, and destructive for failed. Standardize warning treatment once shared tokens exist.
- Count badges are neutral by default. Task count is not failure count; `taskCount > 0` alone does not justify red.
- Toasts use neutral backgrounds and status icons. Errors requiring action need readable explanations and Retry / View details paths rather than a brief flash. Prefer keeping execution feedback in the relevant result region.
- Do not duplicate toasts for normal progress and success. Avoid simultaneously repeating the same event in local status, notifications, and the task panel.

### SQL editor and result grids

- Keep Monaco's theme integration (currently `vs / vs-dark`) and user font-size settings. Syntax highlighting is outside the application decoration color budget; reducing colors must not damage syntax recognition.
- Keep data central: neutral headers, subtle hover, stable selection, and clear sorting / filtering indicators. Preserve virtual scrolling; do not rebuild the whole grid for animation.
- Distinguish `NULL`, empty strings, and booleans. Show true / false or equivalent text / symbols; do not default to red / green boolean mapping.
- Primary keys, unique constraints, and auto-increment are attributes, not warnings or success. Use neutral identifiers such as a key icon or PK / UNIQUE / identity text.
- Show direction and priority for multi-column sorting. Preserve the capability limits in [GRID_SORT_DESIGN.md](docs/GRID_SORT_DESIGN.md): sorting applies to returned results / the current page, not the whole database or SQL re-execution.
- Filters use operator text and removable entries, not a rainbow palette for eq / like / gt and other operators.
- Group formatting, execution plans, copying, and exporting by action. Identify CSV / JSON / INSERT by name. Make export scope and the currently displayed scope explicit.
- Provide details / copying for long text, JSON, and BLOBs rather than leaving an unrecoverable ellipsis. Clearly distinguish editable and read-only grid states.

### AI, transfers, and supporting panels

- AI uses the same neutral surfaces and primary color as the query page. Distinguish user messages, model messages, and code blocks through position, labels, and typography.
- Clearly separate generated SQL awaiting execution from executed results. Execution targets, risk messages, and confirmation follow the ordinary SQL workflow.
- Imports / exports retain the scopes and steps in [TRANSFER_SCOPE_DESIGN.md](docs/TRANSFER_SCOPE_DESIGN.md). Express step state through names, numbers, and progress rather than a color per step.
- Task panels clearly show running, completed, failed, and cancelled states. Keep rows neutral and color only local status signals. Normally running tasks must not appear as error alerts.

### Empty, loading, and error states

| State | Required information | Treatment |
| --- | --- | --- |
| No connections yet | Why the view is empty and how to create a connection | Neutral icon, short explanation, one primary action |
| Not executed yet | Next steps: select a target / enter SQL / execute | Brief guidance; no fabricated sample results |
| Query returned zero rows | Execution succeeded with no matching records | Neutral text; retain duration and available actions |
| Loading / executing | Current action, relevant progress, cancellation capability | Local spinner / skeleton; avoid whole-screen layout shifts |
| Failure | Cause, target context, available actions | Local error icon / text with copyable details |
| Permission / feature restriction | Reason for the restriction and available paths | Clear copy; do not disguise restrictions as system errors |

### Accessibility and copy

- Body text, labels, and button text need at least 4.5:1 contrast. Large text (at least 24px regular or approximately 18.7px bold) needs at least 3:1. Required control boundaries, status graphics, and focus indicators need at least 3:1 against adjacent backgrounds. Decorative separators do not substitute for control-boundary checks.
- Do not communicate state through color alone. Add text / icons and appropriate ARIA semantics for selection, sorting, expansion, and other states.
- Check actual composited colors, especially translucent backgrounds, subdued text, and success / destructive foreground pairs. Existing values document the baseline, not proof of adequate contrast.
- Preserve Radix keyboard and focus behavior. Support Tab, Shift+Tab, Enter, Space, Escape, and arrow keys according to control semantics. Provide suitable keyboard paths for trees, grids, and split panes.
- Use specific verbs and objects. Distinguish “submitted,” “executing,” and “completed”; submitting a request is not successful execution.
- Add copy through the existing `src/lang` system and check long English and Chinese labels. Errors need context and a next step, not generic messages containing only a code.

## Do's and Don'ts

### Do

- Establish information hierarchy and action priority before selecting neutral / primary / semantic roles.
- Reuse tokens, shared components, and existing platform adaptation. Keep semantics consistent across light and dark themes.
- Let SQL, connection context, and data dominate. Use progressive disclosure to reduce initial complexity.
- Verify contrast against actual backgrounds and complete primary workflows with the keyboard.

### Don't

- Do not assign a color to every feature, database type, filter operator, or file format.
- Do not use success green for Execute, error red for ordinary counts, or warning colors for ordinary attributes.
- Do not replace SQLKit's blue theme, force pure-gray neutral surfaces, or switch to monochrome primary buttons to match the reference project.
- Do not add rainbow badges, gradient AI panels, large colored icon containers, or light effects without functional value.
- Do not use historical hardcoding, default Badge styling, or unimplemented tokens as the design basis for new UI.

### Change acceptance checklist

Check each affected surface. Documentation-only changes need source, link, and implementation-consistency checks; they do not require launching the client.

- [ ] Every color has a clear role, with no second decorative accent; each operation context has only one solid primary action.
- [ ] No new hardcoded replacements for semantic tokens; new tokens include light/dark definitions and UnoCSS mappings.
- [ ] Light / dark / system modes work; check editor themes, overlays, disabled, error, selected, and focus states together.
- [ ] Text, buttons, and required interaction signals pass contrast checks; statuses include text or icons.
- [ ] Keyboard access works, focus is visible and restored correctly, and icon buttons have localized accessible names.
- [ ] Empty, loading, zero-row, error, long-content, English / Chinese, and small-window states are clear.
- [ ] macOS / Windows / Linux modifiers, window controls, and drag regions behave correctly.
- [ ] Data scope, sorting scope, and execution status match actual capabilities; visual changes preserve functional semantics.
- [ ] UI implementation changes run `npm run lint:check` and `npm run build`; behavior changes also run relevant tests, and layout changes include actual screenshots at relevant themes and window sizes.

### Existing UI migration priorities (not implemented in this change)

| Priority | Current state | Follow-up |
| --- | --- | --- |
| P1 | Existing foreground pairs such as success / destructive need individual verification | Check contrast on actual backgrounds; adjust semantic pairs when needed while retaining the blue brand |
| P1 | Success / warning in `ui/badge` and notifications still use fixed palettes; a global warning token is missing | Add light/dark semantic tokens and mappings first, then migrate shared components and check all consumers |
| P2 | Some `QueriesPage.vue` toolbar buttons use purple / orange; `QueryResultPanel.vue` assigns sky / amber / green by format | Use neutral toolbars and primary for the main action |
| P2 | `grid/FilterBar.vue` assigns colors to operators; grid booleans and some attributes use red / green / amber | Use neutral text, symbols, and attribute icons; color only actual status |
| P2 | `AppHeader.vue` uses red for ordinary task counts; some connection-page icon containers use multiple colors | Make counts neutral; distinguish connection cards by name / engine icon / actual status |
| P2 | Shared Badge defaults to solid primary; some close buttons appear only on hover | Explicitly select neutral variants for non-interactive badges and make close buttons visible on keyboard focus |
| P2 | Narrow windows and reduced-motion behavior have not been fully checked against this specification | Verify and fix affected workflows; do not claim the current client already satisfies every requirement |

Migrate incrementally by shared component or business area as it changes. Check each change against this specification; establishing the rules does not include a bulk rewrite of business UI.
