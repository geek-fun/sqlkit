import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

/**
 * Composable for inline row data search across all columns.
 *
 * Provides async invocation of the `build_table_search_filter` backend command
 * with a run-ID cancellation pattern to discard stale responses from
 * previous searches.
 *
 * The caller is responsible for debouncing (e.g. 300ms `setTimeout`) and for
 * wiring the returned WHERE clause into the data fetch pipeline.
 */
export function useTableSearch() {
  const isSearching = ref(false)
  let currentRunId = 0

  /**
   * Call the backend to build a search WHERE clause.
   * Returns `null` if the term is empty, no searchable columns exist, or
   * the response was superseded by a newer search.
   */
  async function invokeSearch(
    connectionId: string,
    database: string,
    schema: string | undefined,
    tableName: string,
    searchTerm: string,
  ): Promise<string | null> {
    const term = searchTerm.trim()
    if (!term)
      return null

    const runId = ++currentRunId
    isSearching.value = true

    try {
      const filter = await invoke<string>('build_table_search_filter', {
        connectionId,
        database,
        schema: schema ?? null,
        tableName,
        searchTerm: term,
      })
      // Discard stale response from a previous search
      if (runId !== currentRunId)
        return null

      return filter || null
    }
    catch {
      if (runId !== currentRunId)
        return null
      return null
    }
    finally {
      if (runId === currentRunId)
        isSearching.value = false
    }
  }

  /** Cancel any in-flight search by incrementing the run ID. */
  function cancelPending() {
    currentRunId++
    isSearching.value = false
  }

  return {
    isSearching,
    invokeSearch,
    cancelPending,
  }
}
