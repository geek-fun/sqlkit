/** A single node in the query execution plan tree. */
export type ExplainPlanNode = {
  /** Unique ID within the plan (e.g. "0", "0.1", "0.1.0"). */
  id: string
  /** Display title (e.g. "Seq Scan on users"). */
  title: string
  /** Node type (e.g. "Seq Scan", "Index Scan", "Hash Join"). */
  nodeType: string
  /** Relation/table name. */
  relation?: string
  /** Index name (if applicable). */
  index?: string
  /** Estimated startup cost. */
  startupCost?: number
  /** Estimated total cost. */
  totalCost?: number
  /** Display cost string (e.g. "0.00..35.50"). */
  cost?: string
  /** Estimated number of rows. */
  rows?: number
  /** Actual rows returned (only with EXPLAIN ANALYZE). */
  actualRows?: number
  /** Estimated row width. */
  width?: number
  /** Additional details (e.g. "Filter: (status = 'active')"). */
  details: string[]
  /** Child plan nodes. */
  children: ExplainPlanNode[]
}

/** Parsed explain plan result. */
export type ExplainResult = {
  /** Database type identifier. */
  databaseType: string
  /** Raw output from the backend. */
  raw: string
  /** Format of raw output. */
  format: 'json' | 'text'
  /** Parsed plan tree nodes. */
  nodes: ExplainPlanNode[]
  /** Total estimated cost across all nodes. */
  totalCost?: number
  /** Name of the most expensive node. */
  mostExpensiveNode?: string
  /** Whether this was an EXPLAIN ANALYZE query. */
  analyze: boolean
}
