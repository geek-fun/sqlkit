import type { ExplainPlanNode, ExplainResult } from '@/types/explainPlan'

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/**
 * Parse raw EXPLAIN output into structured ExplainResult.
 */
export function parseExplainResult(raw: string, databaseType: string, format: 'json' | 'text', analyze: boolean): ExplainResult {
  let nodes: ExplainPlanNode[]

  if (format === 'json') {
    if (databaseType === 'postgresql' || databaseType === 'postgres') {
      nodes = parsePostgresJson(JSON.parse(raw))
    }
    else if (databaseType === 'mysql' || databaseType === 'mariadb') {
      nodes = parseMySqlJson(JSON.parse(raw))
    }
    else {
      nodes = parseGenericText(raw)
    }
  }
  else {
    if (databaseType === 'sqlite') {
      nodes = parseSqliteText(raw)
    }
    else if (databaseType === 'mysql' || databaseType === 'mariadb') {
      nodes = parseMySqlAnalyzeText(raw)
    }
    else if (databaseType === 'sqlserver' || databaseType === 'mssql') {
      nodes = parseSqlServerText(raw)
    }
    else if (
      databaseType === 'duckdb'
      || databaseType === 'clickhouse'
    ) {
      nodes = parseGenericText(raw)
    }
    else {
      nodes = parseGenericText(raw)
    }
  }

  const totalCost = calculateTotalCost(nodes)
  const mostExpensiveNode = findMostExpensiveNode(nodes)

  return {
    databaseType,
    raw,
    format,
    nodes,
    totalCost,
    mostExpensiveNode,
    analyze,
  }
}

// ---------------------------------------------------------------------------
// PostgreSQL JSON parser
// ---------------------------------------------------------------------------

/**
 * Recursively parse a PostgreSQL plan object into ExplainPlanNode.
 */
function parsePostgresPlan(plan: Record<string, any>, id: string): ExplainPlanNode {
  const nodeType: string = plan['Node Type'] ?? 'Unknown'
  const relation: string | undefined = plan['Relation Name']
  const index: string | undefined = plan['Index Name']
  const startupCost: number | undefined = plan['Startup Cost']
  const totalCost: number | undefined = plan['Total Cost']
  const rows: number | undefined = plan['Plan Rows']
  const actualRows: number | undefined = plan['Actual Rows']
  const width: number | undefined = plan['Plan Width']

  const cost: string | undefined
    = startupCost !== undefined && totalCost !== undefined
      ? `${startupCost.toFixed(2)}..${totalCost.toFixed(2)}`
      : undefined

  const title = relation ? `${nodeType} on ${relation}` : nodeType

  // Collect details from extra fields
  const details: string[] = []
  const extraFields = [
    'Index Cond',
    'Filter',
    'Join Type',
    'Strategy',
    'Sort Key',
    'Recheck Cond',
    'Hash Cond',
    'Merge Cond',
  ]
  for (const field of extraFields) {
    const val = plan[field]
    if (val !== undefined) {
      details.push(`${field}: ${Array.isArray(val) ? val.join(', ') : String(val)}`)
    }
  }

  // Recurse into child plans
  const childPlans: Record<string, any>[] = plan.Plans ?? []
  const children = childPlans.map((child, i) =>
    parsePostgresPlan(child, `${id}.${i}`),
  )

  return {
    id,
    title,
    nodeType,
    relation,
    index,
    startupCost,
    totalCost,
    cost,
    rows,
    actualRows,
    width,
    details,
    children,
  }
}

/**
 * Parse PostgreSQL EXPLAIN (FORMAT JSON) output.
 */
export function parsePostgresJson(parsed: any): ExplainPlanNode[] {
  if (!Array.isArray(parsed) || parsed.length === 0) {
    return []
  }

  return parsed.map((entry, i) => {
    const plan = entry.Plan ?? entry
    return parsePostgresPlan(plan, String(i))
  })
}

// ---------------------------------------------------------------------------
// MySQL JSON parser
// ---------------------------------------------------------------------------

/**
 * Extract an ExplainPlanNode from a MySQL table object.
 */
function parseMySqlTable(table: Record<string, any>, id: string, extraDetails?: string[]): ExplainPlanNode {
  const nodeType: string = table.access_type ?? 'Unknown'
  const relation: string | undefined = table.table_name
  const index: string | undefined = table.key
  const rows: number | undefined = table.rows_examined_per_scan
  const totalCost: number | undefined = table.cost_info?.query_cost
    ? Number.parseFloat(table.cost_info.query_cost)
    : undefined

  const details: string[] = [...(extraDetails ?? [])]
  if (table.attached_condition) {
    details.push(table.attached_condition)
  }
  if (table.used_columns) {
    details.push(`Columns: ${(table.used_columns as string[]).join(', ')}`)
  }

  const title = relation ? `${nodeType} on ${relation}` : nodeType

  const materialized = table.materialized_from_subquery
  if (materialized) {
    const innerBlock = materialized.query_block as Record<string, any> | undefined
    if (innerBlock) {
      const childNodes = parseMySqlQueryBlock(innerBlock, id)
      if (childNodes.length > 0) {
        return childNodes[0]
      }
    }
  }

  return {
    id,
    title,
    nodeType,
    relation,
    index,
    totalCost,
    rows,
    details,
    children: [],
  }
}

/**
 * Parse a MySQL query_block object, extracting tables from
 * nested_loop, ordering_operation, grouping_operation, or direct table.
 */
function parseMySqlQueryBlock(block: Record<string, any>, baseId: string): ExplainPlanNode[] {
  const nodes: ExplainPlanNode[] = []
  let counter = 0

  // Nested loop join — array of table objects
  const nestedLoop = block.nested_loop as Record<string, any>[] | undefined
  if (nestedLoop) {
    const children: ExplainPlanNode[] = []
    for (const item of nestedLoop) {
      if (item.table) {
        children.push(parseMySqlTable(item.table, `${baseId}.${counter++}`))
      }
    }
    // Wrap in a synthetic join parent node for tree hierarchy
    return [{
      id: baseId,
      title: 'Nested Loop Join',
      nodeType: 'Nested Loop Join',
      details: children.length > 1 ? [] : ['Single table'],
      children,
    }]
  }

  // Ordering operation wraps a table with filesort
  const orderingOp = block.ordering_operation as Record<string, any> | undefined
  if (orderingOp) {
    if (orderingOp.table) {
      const tableNode = parseMySqlTable(
        orderingOp.table,
        `${baseId}.${counter++}`,
        orderingOp.using_filesort ? ['Using filesort'] : undefined,
      )
      nodes.push(tableNode)
    }
    return nodes
  }

  // Grouping operation wraps a table
  const groupingOp = block.grouping_operation as Record<string, any> | undefined
  if (groupingOp) {
    if (groupingOp.table) {
      nodes.push(parseMySqlTable(groupingOp.table, `${baseId}.${counter++}`))
    }
    return nodes
  }

  // Direct table
  const table = block.table as Record<string, any> | undefined
  if (table) {
    nodes.push(parseMySqlTable(table, `${baseId}.${counter++}`))
  }

  return nodes
}

/**
 * Parse MySQL EXPLAIN FORMAT=JSON output.
 */
export function parseMySqlJson(parsed: any): ExplainPlanNode[] {
  const queryBlock = parsed.query_block
  if (!queryBlock) {
    return []
  }

  return parseMySqlQueryBlock(queryBlock, '0')
}

// ---------------------------------------------------------------------------
// SQLite text parser
// ---------------------------------------------------------------------------

/**
 * Parse a single SQLite plan line detail to extract table and index.
 */
function parseSqliteDetail(detail: string): { nodeType: string, relation?: string, index?: string } {
  // Patterns ordered by specificity (longest match first)
  const fullOpMatch = detail.match(/^\d*\s*(USE\s+TEMP\s+B-TREE\s+FOR\s+ORDER\s+BY|EXECUTE\s+CORRELATED\s+SCALAR\s+SUBQUERY)/)
  if (fullOpMatch) {
    return { nodeType: fullOpMatch[1] }
  }

  const tableMatch = detail.match(/^\d*\s*(SCAN\s+TABLE|SEARCH\s+TABLE)\s+(\w+)(?:\s+USING\s+INDEX\s+(\w+))?/)
  if (tableMatch) {
    return {
      nodeType: tableMatch[1],
      relation: tableMatch[2],
      index: tableMatch[3],
    }
  }

  return { nodeType: detail.split(/\s+/)[0] ?? detail }
}

/**
 * Parse SQLite EXPLAIN QUERY PLAN text output.
 * Format: pipe-delimited columns: id|parent|notused|detail
 */
export function parseSqliteText(text: string): ExplainPlanNode[] {
  const lines = text.trim().split('\n').filter(l => l.trim().length > 0)
  if (lines.length === 0) {
    return []
  }

  // Parse all lines first
  const rows = lines.map((line) => {
    const parts = line.split('|').map(s => s.trim())
    if (parts.length < 4) {
      return null
    }
    return {
      id: parts[0],
      parent: parts[1],
      notused: parts[2],
      detail: parts.slice(3).join('|'),
    }
  }).filter((r): r is NonNullable<typeof r> => r !== null)

  if (rows.length === 0) {
    return []
  }

  // Build nodes
  const nodeMap = new Map<string, ExplainPlanNode>()
  for (const row of rows) {
    const { nodeType, relation, index } = parseSqliteDetail(row.detail)
    const node: ExplainPlanNode = {
      id: row.id,
      title: relation ? `${nodeType} on ${relation}` : nodeType,
      nodeType,
      relation,
      index,
      details: [row.detail],
      children: [],
    }
    nodeMap.set(row.id, node)
  }

  // Link children to parents
  const roots: ExplainPlanNode[] = []
  for (const row of rows) {
    const node = nodeMap.get(row.id)!
    if (row.parent === '0') {
      roots.push(node)
    }
    else {
      const parent = nodeMap.get(row.parent)
      if (parent) {
        parent.children.push(node)
      }
      else {
        roots.push(node)
      }
    }
  }

  return roots
}

// ---------------------------------------------------------------------------
// SQL Server text parser
// ---------------------------------------------------------------------------

/**
 * Parse a SQL Server StmtText line to extract operator info.
 */
function parseSqlServerOperator(line: string): { nodeType: string, relation?: string, index?: string, detail: string } {
  // Strip leading |-- and whitespace
  const cleaned = line.replace(/^\s*\|--\s*/, '').trim()

  // Pattern: "OperatorName(OBJECT:([schema].[table]), ...)"
  // Or "OperatorName(Inner Join, ...)"
  const parenIndex = cleaned.indexOf('(')
  const nodeType = parenIndex > 0 ? cleaned.substring(0, parenIndex).trim() : cleaned.trim()

  const detail = cleaned

  // Extract identifiers from OBJECT:([db].[schema].[table]) or OBJECT:([table]) or OBJECT:([table].[index])
  let relation: string | undefined
  let index: string | undefined

  const objMatch = detail.match(/OBJECT:\(([^)]+)\)/)
  if (objMatch) {
    // Split by `].[` or `]` separators
    const parts = objMatch[1]
      .split(/\]\.\[|\]|\[/)
      .map(s => s.replace(/[[\]]/g, '').trim())
      .filter(s => s.length > 0)

    if (nodeType === 'Index Seek' || nodeType === 'Index Scan') {
      // Format: [db].[schema].[table].[index] or [schema].[table].[index]
      if (parts.length >= 3) {
        relation = parts[parts.length - 2] // second-to-last is the table
        index = parts[parts.length - 1] // last is the index name
      }
      else if (parts.length === 2) {
        relation = parts[0]
        index = parts[1]
      }
    }
    else {
      // Table Scan or other operations: [db].[schema].[table] or [table]
      if (parts.length >= 2) {
        relation = parts[parts.length - 1] // last part is the table name
      }
      else if (parts.length === 1) {
        relation = parts[0]
      }
    }
  }

  return { nodeType, relation, index, detail }
}

/**
 * Parse SQL Server SHOWPLAN text output.
 */
export function parseSqlServerText(text: string): ExplainPlanNode[] {
  const lines = text.split('\n').filter(l => l.trim().length > 0)
  if (lines.length === 0) {
    return []
  }

  const nodes: ExplainPlanNode[] = []
  let idCounter = 0

  for (const line of lines) {
    // Only process lines with plan operators
    if (!line.includes('|--')) {
      continue
    }

    const { nodeType, relation, index, detail } = parseSqlServerOperator(line)

    const title = relation ? `${nodeType} on ${relation}` : nodeType

    nodes.push({
      id: String(idCounter++),
      title,
      nodeType,
      relation,
      index,
      details: [detail],
      children: [],
    })
  }

  return nodes
}

// ---------------------------------------------------------------------------
// Generic text parser (DuckDB / ClickHouse)
// ---------------------------------------------------------------------------

/**
 * Parse DuckDB/ClickHouse generic EXPLAIN text output.
 */
export function parseGenericText(text: string): ExplainPlanNode[] {
  const lines = text.split('\n')
    .map(l => l.replace(/[─┐└├│┌┬┤┼╴╵╶╷╸╹╺╻╼╽╾╿■]/g, '').trim())
    .filter(l => l.length > 0 && !l.match(/^[\s\-_.]+$/))

  if (lines.length === 0) {
    return []
  }

  const deduped: string[] = []
  for (const line of lines) {
    if (deduped.length === 0 || line !== deduped[deduped.length - 1]) {
      deduped.push(line)
    }
  }

  return deduped.map((line, i) => {
    const cleanLine = line.replace(/\s+/g, ' ').trim()
    return {
      id: String(i),
      title: cleanLine,
      nodeType: cleanLine.split(/\s+/)[0] ?? cleanLine,
      details: [cleanLine],
      children: [],
    }
  })
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/**
 * Get CSS color class based on cost percentage.
 * Green: < 10%, Yellow: 10-40%, Red: > 40%
 */
export function getCostColor(cost: number, totalCost: number): 'green' | 'yellow' | 'red' {
  if (totalCost <= 0) {
    return 'green'
  }
  const pct = (cost / totalCost) * 100
  if (pct < 10) {
    return 'green'
  }
  if (pct < 40) {
    return 'yellow'
  }
  return 'red'
}

/**
 * Find the most expensive node name.
 */
export function findMostExpensiveNode(nodes: ExplainPlanNode[]): string | undefined {
  let maxCost = -1
  let maxNode: ExplainPlanNode | undefined

  const visit = (node: ExplainPlanNode) => {
    if (node.totalCost !== undefined && node.totalCost > maxCost) {
      maxCost = node.totalCost
      maxNode = node
    }
    for (const child of node.children) {
      visit(child)
    }
  }

  for (const node of nodes) {
    visit(node)
  }

  return maxNode?.title
}

/**
 * Calculate total cost from all root nodes.
 * Recurses into children when a node has no cost itself but has children with costs.
 */
export function calculateTotalCost(nodes: ExplainPlanNode[]): number {
  const visit = (n: ExplainPlanNode): number => {
    if (n.totalCost !== undefined && n.totalCost > 0)
      return n.totalCost
    if (n.children.length > 0)
      return n.children.reduce((s, c) => s + visit(c), 0)
    return 0
  }
  return nodes.reduce((sum, n) => sum + visit(n), 0)
}

/**
 * Flatten tree to array (for summary table).
 */
export function flattenNodes(nodes: ExplainPlanNode[]): ExplainPlanNode[] {
  const result: ExplainPlanNode[] = []

  const walk = (node: ExplainPlanNode) => {
    result.push(node)
    for (const child of node.children) {
      walk(child)
    }
  }

  for (const node of nodes) {
    walk(node)
  }

  return result
}

// ---------------------------------------------------------------------------
// MySQL EXPLAIN ANALYZE TREE text parser
// ---------------------------------------------------------------------------

/**
 * Parse MySQL EXPLAIN ANALYZE TREE-format output.
 * Format: lines prefixed with `->` where indentation indicates nesting depth.
 * E.g.:
 *   -> Nested loop inner join  (cost=1.0 rows=1)
 *       -> Table scan on users  (cost=0.5 rows=2)
 *       -> Index lookup on orders  (cost=0.5 rows=1)
 * Each line may end with "(cost=X rows=Y)" or "(cost=X..Y rows=Z)".
 */
export function parseMySqlAnalyzeText(text: string): ExplainPlanNode[] {
  const lines = text.split('\n').filter(l => l.includes('->'))
  if (lines.length === 0)
    return []

  // Find the base indentation level from the first `->` line
  const baseIndent = Math.min(...lines.map((l) => {
    const idx = l.indexOf('->')
    return idx >= 0 ? idx : Infinity
  }))

  const rootNodes: ExplainPlanNode[] = []
  const stack: { node: ExplainPlanNode, depth: number }[] = []
  let idCounter = 0

  for (const line of lines) {
    const arrowIdx = line.indexOf('->')
    if (arrowIdx < 0)
      continue

    const indent = arrowIdx - baseIndent
    const depth = Math.max(0, Math.round(indent / 4))
    const content = line.substring(arrowIdx + 2).trim()

    // Parse cost and rows from trailing "(cost=... rows=...)"
    const costMatch = content.match(/\(cost=([\d.]+)\s+rows=([\d.]+)\)/)
    const costStr = costMatch?.[1]
    const totalCost = costStr
      ? Number.parseFloat(
          costStr.includes('..') ? costStr.split('..')[1] : costStr,
        )
      : undefined
    const rows = costMatch?.[2] ? Number.parseFloat(costMatch[2]) : undefined

    // Extract details (text before the cost parentheses)
    let nodeType = content
    let details: string[] = []
    if (costMatch) {
      nodeType = content.substring(0, content.indexOf('(cost=')).trim()
      const extra = content.substring(content.indexOf(')') + 1).trim()
      if (extra)
        details = [extra]
    }

    const id = String(idCounter++)
    // Extract relation: everything after " on ", minus any " using ..." suffix
    let relation: string | undefined
    if (nodeType.includes(' on ')) {
      const afterOn = nodeType.split(/ on /i).slice(1).join(' on ')
      const usingIdx = afterOn.search(/ using /i)
      relation = usingIdx >= 0 ? afterOn.substring(0, usingIdx).trim() : afterOn
      if (usingIdx >= 0) {
        details = [...details, afterOn.substring(usingIdx).trim()]
      }
    }
    const node: ExplainPlanNode = {
      id,
      title: nodeType,
      nodeType: nodeType.split(/ on /i)[0] || nodeType,
      relation,
      totalCost,
      cost: costStr,
      rows,
      details,
      children: [],
    }

    // Build tree from depth stack
    while (stack.length > 0 && stack[stack.length - 1].depth >= depth)
      stack.pop()

    if (stack.length === 0) {
      rootNodes.push(node)
    }
    else {
      stack[stack.length - 1].node.children.push(node)
    }

    stack.push({ node, depth })
  }

  return rootNodes
}
