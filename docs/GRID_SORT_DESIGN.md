# Result Grid Sorting — Client-Side Primitive Sort Design

> **Status**: Implemented
> **Date**: 2026-10-08
> **Scope**: `DataGrid` (query results) and `DataTableView` (table data browser)

## Executive Summary

The result grids used to sort by **re-executing the user's SQL** on the backend
(`execute_sorted_query` → `SELECT * FROM (…) ORDER BY …`). That path only works
for plain parsable `SELECT`s, re-runs potentially expensive queries on every
header click, and gave the table browser no sort at all.

This design moves sorting to the **frontend, over the rows already returned**.
It is instant, works for any result set, and is deliberately limited to
**primitive columns** (text / number / boolean / date / NULL). JSON documents,
arrays and BLOBs stay unsortable because they have no meaningful client-side
total ordering.

```
header click ──▶ useDataGridSort (SortState) ──▶ sortRowsByState(rows)
                                                   │
                                    primitive-only comparator
                                                   ▼
                                             sortedRows (computed)
                                                   ▼
                                    virtual rows / table rows render
```

## 1. What counts as sortable

A column is sortable when **both** hold:

1. Its declared SQL type is primitive —
   `/^(?:JSONB?|ARRAY|BYTEA|BLOB|BINARY|VARBINARY|IMAGE|GEOMETRY|GEOGRAPHY|HSTORE|XML)/i`
   disqualifies it.
2. Every returned cell is primitive (`string | number | boolean | null | undefined`).
   A single object/array cell disqualifies the whole column, because a mixed
   column cannot be ordered consistently.

Non-sortable headers render as plain labels with a tooltip; the header context
menu disables its sort entries and explains why.

## 2. Comparison rules

`compareSortValues(a, b, columnType)` resolves an ascending order:

| Kind | Detection | Ordering |
| --- | --- | --- |
| number | numeric SQL type or `typeof value === 'number'` | `Number()` compare; numeric strings (`bigint`) compare numerically |
| boolean | `BOOL`/`BIT` type or boolean value | `false < true`; `t/f/1/0/y/n/yes/no` accepted |
| date | `DATE`/`TIMESTAMP`/`TIME` type | `Date.parse` when both parse, otherwise string order |
| string | fallback | `localeCompare(…, { numeric: true, sensitivity: 'base' })` |

- **NULL/undefined** sort after real values in ascending order. Descending is a
  pure reversal, so NULLs come first — matching PostgreSQL's default.
- Comparisons never inspect non-primitive values; those columns are filtered out
  before any comparison happens.

## 3. Sorting state

`useDataGridSort` owns an ordered `SortState = { column, direction }[]`:

- Click (no modifier): ASC → DESC → cleared, replacing any existing rules.
- Shift-click: append, cycling ASC → DESC → removed (multi-column sort).
- Context menu: `setSort(column, direction)` sets one direction while keeping
  other rules.
- `pruneSort(validColumns)` drops rules for columns that disappeared or became
  unsortable (e.g. after running a different query).

`sortRowsByState(rows, sortState, columnTypes)`:

- returns a **new array** (input is never mutated),
- applies every rule in priority order,
- is stable: ties keep their original index,
- short-circuits when there are no rules or no rows.

## 4. Component wiring

### `DataGrid` (query results)

- `sortedRows` is the single source of truth for rendering, row actions,
  copy/export and the status bar.
- Header clicks call `handleHeaderSort`; the context menu is told `sortable`.
- Selection is cleared whenever rows, columns or the sort state change, because
  selection is positional.
- The status bar shows `id ↑, name ↓` for active rules.
- `DataGrid` no longer emits `sortChange`; sorting is internal.

### `QueryResultPanel`

- Sort no longer round-trips. `reExecuteFiltered` only re-executes for filters,
  which still change the result set itself.
- `activeSort`, `handleSortChange` and the `@sort-change` binding are gone.

### `DataTableView` (table browser)

- Same composable and `sortedRows`, scoped to the page that was fetched
  (`get_table_data` paginates server-side).
- Sort resets when the table/connection changes and survives page changes.
- CSV export and batch delete read from `sortedRows`, so they match the view.

## 5. Non-goals / limitations

- **No global sort in the table browser.** The current page is sorted; a
  cross-page sort would need `ORDER BY` pushed into `get_table_data`.
- **No sorting of JSON/BLOB columns.** Type-aware ordering for those types is a
  database concern, not a client one.
- The backend `execute_sorted_query` still accepts `sort` rules; the frontend
  simply always sends `[]` now.

## 6. Testing

- `src/__tests__/gridSort.test.ts` — sortability detection, per-kind
  comparison, NULL placement, multi-column priority, stability, immutability.
- `src/__tests__/useDataGridSort.test.ts` — click/shift cycling, `setSort`,
  `pruneSort`.
