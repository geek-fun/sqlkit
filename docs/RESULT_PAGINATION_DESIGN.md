# Application Pagination

Table browsing and query results share `PaginationControls` and `usePagination`.
The footer, localized labels, first/previous/next/last controls, direct page entry,
page-size presets and custom-size validation use the same implementation.
Every view starts at 100 rows and returns to page 1 when its size changes.
Choosing the current size is a no-op. The footer wraps without clipping its
controls in constrained panels, and remains visible for empty/loading/error states.
The full database name remains visible, wrapping even without spaces, with a
divider before row counts and timing. Context, metrics and navigation wrap as
separate groups when space is constrained; long names never paint over the
result statistics.

Table browsing fetches one page from the database using a numeric limit and
offset. It offers the same numeric sizes and custom entry, capped at the backend's
unsigned 32-bit limit, without a show-all action. Refresh obtains the count first,
clamps the page if rows were removed, and fetches the corrected offset. Filters
and table changes reset to page 1. Navigation clears positional selection and row
dialogs; completed fetches reset vertical scrolling while retaining horizontal
scrolling. Loading and error states disable pagination consistently.

Query results are paged locally over the rows returned by the execution command.
Page navigation never executes SQL, so it also works for script results and
statements that cannot be wrapped in a SELECT. Pagination does not reduce the
number of rows fetched by the backend or bypass a SQL LIMIT or driver row limit.

- Start at 100 rows per page. Offer 50, 100, 200, 500, and 1000 rows, a positive
  safe-integer custom size, and all returned rows.
- Provide first, previous, next, and last page buttons plus direct page entry.
  Enter or blur applies a valid page, numbers past the end clamp to the last page,
  and invalid input or Escape restores the current page.
- Display the visible range and total returned rows. Empty results show 0–0 of 0
  on page 1 of 1. Truncated results show the loaded count with a limited-result
  label rather than claiming a complete database count.
- A page-size change starts at page 1. Replacing results, including a filter
  response or new execution, starts at page 1 while retaining the current size.
  Switching script results creates a fresh grid with the default page size.
- Sort primitive values within the current page using the existing grid sort
  rules. Copy / export all includes the entire returned result in execution
  order; selected-row and individual-row actions use the visible page.
- Clear positional selection, row context menus, and row dialogs when the page
  changes. Reset vertical scrolling to the top and preserve horizontal scrolling.
- Keep the footer outside the row viewport so it remains visible during loading,
  errors, and empty results. Disable controls during loading and errors. Footer
  groups wrap in constrained panels; table overflow stays inside the grid.
- Use shared neutral buttons, inputs, and portalled menus. Localize accessible
  names and labels in English and Chinese, retaining visible keyboard focus.
