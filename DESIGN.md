---
name: SQLKit
description: A restrained, dense, cross-platform database workspace centered on SQL and data
colors:
  primary-light: "hsl(221.2 83.2% 45%)"
  primary-dark: "hsl(221.21 83.19% 53.33%)"
  background-light: "hsl(0 0% 100%)"
  background-dark: "hsl(220.00 12.00% 9.80%)"
  foreground-light: "hsl(222.2 84% 4.9%)"
  foreground-dark: "hsl(217.50 17.39% 90.98%)"
  card-light: "hsl(0 0% 100%)"
  card-dark: "hsl(220.00 11.39% 15.49%)"
rounded:
  lg: "8px"
  md: "6px"
  sm: "4px"
---

# SQLKit Design Specification

## Overview

**Design direction: A quiet SQL workspace.** Users should notice connection context, SQL, and query results first, actions second, and brand expression last. Support long sessions of writing, inspecting, and comparing data through high information density, clear hierarchy, stable layouts, and predictable interactions.

This file defines the design constraints for new and modified SQLKit client UI, alongside [AGENTS.md](AGENTS.md). Preserve the existing blue theme, light / dark / system modes, compact desktop layout, and shadcn-vue / Radix Vue component system. The graphite theme and semantic color migration apply to the client shell, shared controls, SQL editor, connections, grids, transfers, settings, and AI surfaces.

### Design principles

The client uses a restrained desktop workspace with softly cool graphite surfaces in dark mode and blue primary actions in both themes.

- Give colors fixed semantics: blue for interaction, green for actual success, red for errors and danger, amber for actual warnings, and neutrals for ordinary information.
- Keep accents scarce and let typography, spacing, and surface hierarchy organize the workspace.
- Use shared semantic tokens for light, dark, and system appearance, including the SQL editor and overlays.
- Preserve compact layouts, progressive disclosure, predictable keyboard access, and readable contrast.
- Keep decorative gradients, ambient effects, and glass treatments out of the database workspace.

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
| `--background` | `0 0% 100%` | `220.00 12.00% 9.80%` | Workspace and editor |
| `--chrome` | `210.00 40.00% 98.04%` | `217.50 12.50% 12.55%` | Title bar, navigation, sidebar, toolbars |
| `--foreground` | `222.2 84% 4.9%` | `217.50 17.39% 90.98%` | Body text and data |
| `--card` | `0 0% 100%` | `220.00 11.39% 15.49%` | Containers |
| `--popover` | `0 0% 100%` | `220.00 11.39% 15.49%` | Menus, dialogs and editor widgets |
| `--primary` | `221.2 83.2% 45%` | `221.21 83.19% 53.33%` | Solid primary actions |
| `--primary-foreground` | `0 0% 100%` | `0.00 0.00% 100.00%` | Text on primary fills |
| `--primary-text` | `224.28 76.33% 48.04%` | `218.46 100.00% 77.06%` | Links and small active indicators |
| `--selection` | `220.00 100.00% 95.29%` | `215.71 35.59% 23.14%` | Text and grid selection |
| `--secondary` | `210 40% 96.1%` | `220.00 11.39% 15.49%` | Secondary surfaces |
| `--muted` | `210 40% 96.1%` | `220.00 11.39% 15.49%` | Subdued surfaces |
| `--muted-foreground` | `213.33 16.82% 41.96%` | `217.89 11.80% 68.43%` | Supporting text |
| `--accent` | `210 40% 96.1%` | `216.92 13.13% 19.41%` | Neutral hover and selected controls |
| `--border` | `214.3 31.8% 91.4%` | `217.50 13.33% 23.53%` | Structural separation |
| `--input` | `213.75 13.91% 54.90%` | `215.56 11.79% 44.90%` | Control boundaries |
| `--ring` | `215.38 16.32% 46.86%` | `217.89 11.80% 68.43%` | Keyboard focus |
| `--destructive` | `0.00 73.71% 41.76%` | `0.00 53.53% 47.25%` | Solid destructive actions |
| `--destructive-foreground` | `0.00 0.00% 100.00%` | `0.00 0.00% 100.00%` | Text on destructive fills |
| `--destructive-text` | `0.00 73.71% 41.76%` | `4.82 81.16% 72.94%` | Readable error text and icons |
| `--destructive-muted` | `0.00 85.71% 97.25%` | `352.94 19.10% 17.45%` | Subdued error feedback |
| `--destructive-border` | `0.00 96.30% 89.41%` | `3.40 29.61% 35.10%` | Error feedback boundary |
| `--success` | `142.43 71.81% 29.22%` | `143.02 47.25% 64.31%` | Success text and connected/completed signals |
| `--success-foreground` | `0.00 0.00% 100.00%` | `220.00 12.00% 9.80%` | Text on success fills |
| `--success-muted` | `138.46 76.47% 96.67%` | `148.42 22.89% 16.27%` | Success feedback surface |
| `--success-border` | `141.00 78.95% 85.10%` | `141.67 23.08% 30.59%` | Success feedback boundary |
| `--warning` | `35.65 100.00% 27.06%` | `36.61 79.73% 70.98%` | Actual warnings and risk signals |
| `--warning-foreground` | `0.00 0.00% 100.00%` | `220.00 12.00% 9.80%` | Text on warning fills |
| `--warning-muted` | `39.00 100.00% 96.08%` | `36.67 20.45% 17.25%` | Warning feedback surface |
| `--warning-border` | `39.10 67.94% 74.31%` | `37.36 32.12% 32.35%` | Warning feedback boundary |

Dark mode uses the approved softly cool graphite palette. Preserve blue primary actions. Use `primary-text` for blue text and icons, and `destructive-text` for error text and icons; solid fill tokens are not readable-text tokens. `success` and `warning` are paired with dark foregrounds in dark mode and white foregrounds in light mode.

### Semantic ownership

| Visual signal | Allowed meaning | Must not represent |
| --- | --- | --- |
| Blue `primary / primary-text` | Primary actions, links, selected controls, small active indicators | Success, ordinary badges, large panel backgrounds |
| Neutrals `foreground / muted / secondary / accent` | Body text, supporting information, toolbars, navigation backgrounds, layout hierarchy, disconnected states | Important errors or risks hidden through low contrast |
| Green `success` | Successful connection tests, connected states, successful execution, completed transfers | Filled Execute buttons or decorative selling points; existing green recognition cues are independent of this status role |
| Red `destructive / destructive-text` | Execution failures, connection failures, deletion confirmations | Disconnected states or decoration; existing false-value cues are independent of this status role |
| Amber `warning` | Actual warnings, risks, exceptional states requiring attention | New decorative fills; existing key, format, and unsaved cues retain their recognition colors |

Preserve established icon and data colors during surface or theme refactors. Use neutral text for ordinary prose; recognition colors for actions, types, attributes, and values remain useful and must not be flattened to gray. SQLKit's `primary` already owns blue interaction emphasis. Warning includes foreground, muted-background, and border pairs in both themes. There is no `--info` mapping; ordinary information remains neutral. Add tokens only for semantics that require a distinct role.

### Color budget

1. **Start with neutrals, then add emphasis.** Most backgrounds, borders, body text, and icons in a resting workspace use existing neutral tokens. Establish hierarchy through typography, spacing, and position first.
2. **Aim for accents to occupy no more than roughly 10% of the UI.** This is a review heuristic for the app shell, toolbars, panels, and forms, not a pixel-counting requirement. Evaluate SQL syntax highlighting, actual data charts, and local error feedback separately for readability; these exceptions must not expand decorative color use.
3. **Use at most one solid primary action per operation context.** Use a solid action only when the context benefits from it, such as Save in a form dialog. Query toolbar actions, including Execute, use compact ghost buttons with the original colored icons and subtle hover surfaces. An independent dialog may have its own primary action. Focus rings and status dots do not count as a second solid primary action.
4. **Success, warning, and failure may coexist when backed by actual state.** Default to an icon / status dot with text, adding a local, subdued background when needed. Do not tint an entire row, card, or panel.
5. **Preserve information-bearing color.** Existing object, action, value, attribute, filter, and vendor cues are part of the product vocabulary. Keep their original hues and combine them with distinct glyphs or labels. Use restraint on large surfaces rather than removing these cues.

### Allowed and avoided treatments

| Context | Default treatment | Avoid |
| --- | --- | --- |
| Query toolbar | Small colored action icons, ghost buttons, subtle hover feedback | Solid blue Execute tiles or colored button backgrounds |
| Navigation / tabs | Neutral backgrounds and font weight; small blue indicators when needed | Solid saturated blue blocks or multicolor navigation entries |
| Attribute badges | Explicitly select `outline` or `secondary` on `Badge` | Solid blue badges chosen merely because that variant is the default |
| Connection cards | Name, engine icon, status text / dot | Extending small vendor icon colors across entire connection cards |
| Alerts / notifications | Neutral surfaces, status icons, readable text | Large saturated red or green backgrounds |
| AI assistant / Ultimate indicators | The same neutral containers and small primary accents | A separate purple theme, rainbow gradients, glowing borders |

### Color exceptions

- Preserve the object browser's existing palette: green table groups and objects, purple views, indigo materialized views, blue procedures, amber functions, yellow database icons, and sky schema folders. Keep the original category/leaf shades. Table groups use `table-properties`, table leaves use `table`, and materialized views use `layers`, so identification also has a shape cue.
- Preserve the query toolbar's original emerald Execute, violet EXPLAIN, amber Format, and sky Save colors, including their light/dark variants and existing ANALYZE state. Execute stays a compact ghost button with no persistent solid primary fill. Accessible labels, pressed semantics, and distinct glyphs remain required.
- Preserve existing colors for boolean values, primary/foreign keys, uniqueness, identity, filter operators, execution-plan details, permission levels, transfer roles, favorites, and unsaved markers. These encode content or recognition independently of operational success/failure. Keep the existing labels and symbols alongside color.
- Result copy/export icons retain their established sky CSV, amber JSON, and emerald INSERT colors. Preserve format labels and scope information.
- Existing palette utilities for these recognition cues remain valid. This surface refactor must not replace them with a new palette or collapse them to neutral tokens. A future migration to shared recognition tokens must preserve the existing hues and light/dark variants and be reviewed separately.
- Monaco SQL syntax highlighting, code diffs, and actual data charts may use multiple colors when those colors explain content. Do not extend their palettes into application controls.
- Existing database vendor logos may retain identifying colors inside small icons. Do not extend those colors to cards, buttons, or navigation backgrounds.
- Charts should emphasize the series of interest with primary and use neutrals for the others. When multiple series need distinction, a dedicated chart palette is allowed with labels, legends, or line styles. `--chart-1` through `--chart-5` exist in CSS but are not registered as UnoCSS color names; do not assume `text-chart-1` works.
- Data diffs may use success / destructive for additions and deletions, accompanied by `+ / −` or text. Boolean recognition colors describe values independently of success or failure.

### Token implementation process

1. Check whether an existing role can express the meaning before considering opacity variants. Opacity does not waive contrast requirements: text must remain readable against the actual composited background.
2. Do not add hex / RGB / HSL literals or palette utilities such as `blue-500`, `green-600`, and `violet-*` in business components to replace existing semantic roles. Preserve established recognition utilities described above; isolate new chart, syntax, and vendor-asset exceptions in their respective modules.
3. When adding a semantic token, include all of the following in the same change: `:root` and `.dark` definitions in `src/assets/index.css`, necessary foreground / subdued-background pairs, `uno.config.ts` mappings, shared component variants where needed, light/dark verification, and this document's update.
4. Do not submit UI that depends on unimplemented utilities such as `text-warning` or `bg-info` before their mappings exist. Use `warning` for actual risk; preserve the existing attribute recognition palette.
5. Check all consumers before changing shared tokens. Preserve the approved graphite surface hierarchy and blue brand when adjusting individual semantic colors.

## Typography

### Fonts and hierarchy

The global body size is `var(--font-size-form)` (14px), and `src/assets/index.css` does not define a custom brand font. Keep system sans-serif fonts with platform Chinese fallbacks. Do not introduce decorative fonts or remote font dependencies for the client. Keep Monaco's monospace editor font; SQL, types, object identifiers, and code snippets may use the existing `font-mono` utility.

The type scale is token-driven. `index.css` defines `--font-size-caption / dense / body / form / title / page-title` (11 / 12 / 13 / 14 / 16 / 20px); `uno.config.ts` maps the two sizes missing from the default scale (`text-caption` = 11px, `text-body` = 13px), and the rest reuse default utilities (`text-xs` = 12, `text-sm` = 14, `text-base` = 16, `text-xl` = 20). Business components must use these utilities or the CSS variables — never a bare px/rem font size. Monaco's editor font size remains a user setting.

The following hierarchy constrains new UI; it does not require rewriting all existing page titles.

| Role | Token / utility | Usage |
| --- | --- | --- |
| Page title | `text-xl` (20px / 600) | Standalone pages such as Connections and Settings; do not add large headings to editor workspaces |
| Dialog / panel / card title | `text-base` (16px / 600) | Dialog titles, side-panel headers, card and section titles |
| Body / form | `text-sm` (14px / 400) | Descriptions, inputs, buttons |
| Sidebar lists and menus | `text-body` (13px / 400–500) | Connection selector rows, saved-query names, dropdown items outside dense surfaces |
| Dense data / supporting labels | `text-xs` (12px / 400–500) | Result grids, object trees, query tabs, toolbars, status bars |
| Captions and badges | `text-caption` (11px / 500) | Attribute badges, timestamps, task counters; the readable lower bound — 9–10px text is forbidden |

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
| Feature navigation | 56px wide, 36 × 36px entries, 20px icons | `AppSidebar.vue` |
| Query sidebar | Initially 250px, resize range 200–400px | `QueriesPage.vue` |
| Object tree rows | 24px row height, 12px text, 14px icons | `SchemaTree.vue` |
| Query tabs | 35px high, maximum 200px per tab | `QueryTabs.vue` |
| Result toolbar | 36px high (result panel and grid toolbars share this) | `QueryResultPanel.vue`, `DataGrid.vue` |
| Standard buttons / inputs | 32px high; 24 / 28 / 36px via `xs` / `sm` / `lg` | `ui/button`, `ui/input` |
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
- Icon scale: 12px for inline glyphs (chevrons, close buttons, badge icons), 14px for tree, tab, and compact toolbar icons, 16px for standard buttons, menu items, and input adornments, 20px for main navigation. Dropdown-menu icons inherit the shared `size-4` from `DropdownMenuItem`; do not override per menu. The visible icon is smaller than its hit target; keep standard action targets at 32px and compact grid actions at least 24px with sufficient spacing and keyboard access.
- Database vendor logos use three sizes: 14px inside lists and tabs, 16px in dropdown selected states and forms, 20px on cards and landing states (`DbTypeIcon` defaults to 16).
- Status dots use 8px (`h-2 w-2`) inline within rows and 10px (`h-2.5 w-2.5`) as a standalone indicator beside a control.
- Reuse the component's existing Carbon / Lucide style. Do not add icon libraries for color or decoration, or mix emoji into an action group.

## Components

### Shared components and states

Prefer `src/components/ui/*`, importing through each module's `index.ts`. Manage shared variants with CVA; business components select roles and states. Keep icons, labels, and interactions consistent for the same action across pages.

Interactive components must cover default, hover, focus-visible, active / selected, and disabled states. Async actions also need loading, success, and error states. Hover signals interactivity, selected signals selection, and focus-visible signals keyboard position; do not collapse all three into the same blue background.

### Buttons and toolbars

- `default`: the single primary action, retaining `bg-primary text-primary-foreground`. Heights: `xs` 24px, `sm` 28px, default 32px, `lg` 36px; icon buttons mirror this via `icon-xs` / `icon-sm` / `icon`. Use `sm` for sidebar and toolbar rows instead of overriding heights per component.
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

- Settings opens on General, grouping theme, application updates, and Account & Plan. Keep account controls in this first category, with About at the bottom after Account & Plan. About reads the running Tauri app version, with the package version as a browser fallback. Opening General checks for updates without starting installation; show checking, available, current, and failure states beside the version. Keep repository and website links; omit unmaintained community links.
- Reuse 32px Input / Select controls and the existing label / helper / error structure. Placeholders do not replace labels.
- Group host, port, authentication, database, and related fields by task. Progressively reveal advanced SSL / SSH settings, retaining the conditional-field behavior in [SSL_UI_DESIGN.md](docs/SSL_UI_DESIGN.md).
- Place validation near the field and explain the cause and remedy. Preserve entered content after failure; handle sensitive values under existing security policies.
- Give dialogs a clear title, a description matching the content, and reachable close / cancel controls. Manage focus, Escape, and focus restoration after closing.
- Destructive confirmations for deleting data, DROP / TRUNCATE, and similar actions must show the target and impact scope. Describe actual capabilities; do not invent recovery or rollback guarantees.

### Badges, notifications, and task states

- Non-interactive attributes default to `Badge variant="outline"` or `secondary`. Status uses a dot / icon with text. The shared Badge defaults to neutral secondary. Explicitly use success / warning / destructive variants only for real states; those variants use subdued semantic pairs.
- Use success for connected / completed, neutrals for disconnected / not started, and destructive for failed. Use the shared warning tokens for actual risks.
- Count badges are neutral by default. Task count is not failure count; `taskCount > 0` alone does not justify red.
- Toasts use neutral backgrounds and status icons. Errors requiring action need readable explanations and Retry / View details paths rather than a brief flash. Prefer keeping execution feedback in the relevant result region.
- Do not duplicate toasts for normal progress and success. Avoid simultaneously repeating the same event in local status, notifications, and the task panel.

### SQL editor and result grids

- Keep the full database name visible in result and table-data footers. Use a divider and wrap context, row statistics, and pagination controls independently; do not truncate the name or let it overlap statistics.
- Result column headers show the database-provided type on the same line as the name when metadata is available, using small neutral text. Keep numeric headers right-aligned, truncate long types with the full value available on hover, and omit type labels when metadata is absent. Sortable columns always show a neutral sort indicator, with directional arrows for active sorting; do not hide it until hover.
- Keep the result-set selector inside the existing result toolbar. Collapse secondary export actions into a menu at narrow panel widths to preserve a single toolbar row.
- Use the shared pagination footer and state model in table browsing and query result grids, starting at 100 rows per page. Keep first / previous / next / last, direct page entry, common page-size presets, custom sizes, and the row range consistent. Wrap footer groups at narrow widths; keep navigation reachable for empty results and disable it while loading or showing an error. Table browsing fetches one page at a time; query grids page returned rows locally and additionally offer all returned rows. Copy / export retains its stated data scope. See [RESULT_PAGINATION_DESIGN.md](docs/RESULT_PAGINATION_DESIGN.md).
- Use the token-driven `sqlkit-light / sqlkit-dark` Monaco themes and preserve user font-size settings. Both inherit Monaco's built-in grammar support; editor, gutter, active-line, selection, completion and hover surfaces read the active CSS theme. Syntax hues live in `--syntax-keyword / string / number / comment` in the same CSS token source. Syntax highlighting is outside the application decoration color budget; reducing colors must not damage syntax recognition.
- Keep data central: neutral headers, subtle hover, stable selection, and clear sorting / filtering indicators. Preserve virtual scrolling; do not rebuild the whole grid for animation.
- Distinguish `NULL`, empty strings, and booleans. Preserve the existing green true / red false colors and check / cross symbols; color supplements the value glyphs.
- Preserve existing key / constraint / auto-increment colors and PK / UNIQUE / identity labels. These attribute cues do not imply operational warnings or success.
- Show direction and priority for multi-column sorting. Preserve the capability limits in [GRID_SORT_DESIGN.md](docs/GRID_SORT_DESIGN.md): sorting applies to returned results / the current page, not the whole database or SQL re-execution.
- Filters retain their existing operator colors, operator text, and removable entries so repeated conditions remain easy to scan.
- Group formatting, execution plans, copying, and exporting by action. Identify CSV / JSON / INSERT by name. Make export scope and the currently displayed scope explicit.
- Provide details / copying for long text, JSON, and BLOBs rather than leaving an unrecoverable ellipsis. Clearly distinguish editable and read-only grid states.

### AI, transfers, and supporting panels

- AI uses the same neutral surfaces and primary color as the query page. Distinguish user messages, model messages, and code blocks through position, labels, and typography.
- Clearly separate generated SQL awaiting execution from executed results. Execution targets, risk messages, and confirmation follow the ordinary SQL workflow.
- Imports / exports retain the scopes and steps in [TRANSFER_SCOPE_DESIGN.md](docs/TRANSFER_SCOPE_DESIGN.md). Preserve step recognition colors alongside names and numbers; express execution state through progress and status labels.
- Task panels clearly show running, completed, failed, and cancelled states. Keep rows neutral and preserve local role icons and status signals. Normally running tasks must not appear as error alerts.

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

- Do not add decorative color to large feature surfaces or remove established type, filter, or format recognition cues during theme work.
- Do not turn recognition colors into large filled actions or infer execution status from attribute/value colors. Preserve the compact Execute action and existing icon colors.
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

### Theme maintenance

- Keep the CSS token source, UnoCSS mappings, Monaco adapter, and this specification synchronized.
- Shared badges default to neutral secondary. Preserve established icon, database-attribute, boolean, filter, permission, format, and vendor colors during theme changes. Use explicit labels or symbols alongside them. Surface tokens and readable semantic feedback remain coordinated across themes.
- Theme preview tiles scope the same light/dark tokens instead of maintaining a second miniature palette.
- Automated contrast checks cover text on workspace, chrome, raised, selection, and semantic feedback surfaces, plus foregrounds on solid actions. Editor theme tests cover light/dark switching and token inheritance.
- Continue to verify real content, narrow windows, keyboard focus, system appearance changes, and native window controls on the target operating systems. Theme tests do not replace native-platform interaction checks.

### AI permission controls

Per-connection permission editing uses the existing local Ultimate gate and inline upgrade prompt; Community connection edits preserve saved permission decisions. Connection dialogs group the agent prompt with a collapsed AI permissions section shared by Data Studio and external MCP, separate from SSL and SSH tunnel blocks. Default to system inheritance, show effective allowed actions, and offer visible radio choices for access levels, including Custom. A short description below the option group reflects the selected access level. Custom reveals per-action switches whose labels show Use default (decision) for inherited actions and the explicit decision otherwise. Presets and custom editing change the same permission values and do not form separate policy layers. The system settings page edits action defaults, without a separate connection allowlist or MCP exposure switch. Separate shared AI defaults from MCP service settings using the existing settings cards. Use switches for binary actions, and preserve shared RadioGroup, Switch, Label, Button, neutral surfaces, localized English/Chinese copy, keyboard focus, and loading/error/retry states. See [AI permissions](docs/AI_PERMISSIONS.md) for enforcement and migration behavior.

Data Studio tool outcomes remain visible outside collapsible result details. Policy blocks use a local destructive notice naming the connection, required action, and recorded permission source; user rejection uses warning feedback and explicit rejection copy. Neither uses a success verb. A blocked conversation shows a paused status near the composer. Review permissions edits the blocked connection using the existing connection permission control, including inherited denials, with Save and Back to chat in one footer row. Global defaults remain in Settings. Retry request starts a fresh checked turn. Keep these notices and outcomes when restoring history; an unknown or interrupted outcome must not appear successful.
