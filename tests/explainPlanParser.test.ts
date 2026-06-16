import type { ExplainPlanNode } from '@/types/explainPlan'
import {
  calculateTotalCost,
  findMostExpensiveNode,
  flattenNodes,
  getCostColor,
  parseExplainResult,
  parseGenericText,
  parseMySqlAnalyzeText,
  parseMySqlJson,
  parsePostgresJson,
  parseSqliteText,
  parseSqlServerText,
} from '@/utils/explainPlanParser'

// ---------------------------------------------------------------------------
// PostgreSQL JSON parser tests
// ---------------------------------------------------------------------------

describe('parsePostgresJson', () => {
  it('parses a simple Seq Scan', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Seq Scan',
          'Relation Name': 'users',
          'Startup Cost': 0.00,
          'Total Cost': 35.50,
          'Plan Rows': 250,
          'Plan Width': 36,
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('Seq Scan')
    expect(n.relation).toBe('users')
    expect(n.startupCost).toBe(0.00)
    expect(n.totalCost).toBe(35.50)
    expect(n.rows).toBe(250)
    expect(n.width).toBe(36)
    expect(n.cost).toBe('0.00..35.50')
    expect(n.children).toHaveLength(0)
  })

  it('parses an Index Scan with filter', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Index Scan',
          'Relation Name': 'orders',
          'Index Name': 'idx_orders_status',
          'Startup Cost': 0.25,
          'Total Cost': 12.75,
          'Plan Rows': 50,
          'Plan Width': 24,
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('Index Scan')
    expect(n.relation).toBe('orders')
    expect(n.index).toBe('idx_orders_status')
    expect(n.startupCost).toBe(0.25)
    expect(n.totalCost).toBe(12.75)
  })

  it('parses a Nested Loop join with two children', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Nested Loop',
          'Startup Cost': 0.50,
          'Total Cost': 95.00,
          'Plan Rows': 100,
          'Plan Width': 48,
          'Plans': [
            {
              'Node Type': 'Seq Scan',
              'Relation Name': 'users',
              'Startup Cost': 0.00,
              'Total Cost': 35.50,
              'Plan Rows': 250,
              'Plan Width': 36,
            },
            {
              'Node Type': 'Index Scan',
              'Relation Name': 'orders',
              'Index Name': 'idx_orders_user',
              'Startup Cost': 0.15,
              'Total Cost': 0.30,
              'Plan Rows': 1,
              'Plan Width': 12,
            },
          ],
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const parent = nodes[0]
    expect(parent.nodeType).toBe('Nested Loop')
    expect(parent.children).toHaveLength(2)
    expect(parent.children[0].nodeType).toBe('Seq Scan')
    expect(parent.children[0].relation).toBe('users')
    expect(parent.children[1].nodeType).toBe('Index Scan')
    expect(parent.children[1].relation).toBe('orders')
  })

  it('parses a Hash Join with two children', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Hash Join',
          'Join Type': 'INNER',
          'Startup Cost': 5.00,
          'Total Cost': 200.00,
          'Plan Rows': 500,
          'Plan Width': 72,
          'Plans': [
            {
              'Node Type': 'Seq Scan',
              'Relation Name': 'customers',
              'Startup Cost': 0.00,
              'Total Cost': 45.00,
              'Plan Rows': 300,
              'Plan Width': 36,
            },
            {
              'Node Type': 'Hash',
              'Startup Cost': 80.00,
              'Total Cost': 80.00,
              'Plan Rows': 200,
              'Plan Width': 36,
              'Plans': [
                {
                  'Node Type': 'Seq Scan',
                  'Relation Name': 'orders',
                  'Startup Cost': 0.00,
                  'Total Cost': 80.00,
                  'Plan Rows': 200,
                  'Plan Width': 36,
                },
              ],
            },
          ],
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const join = nodes[0]
    expect(join.nodeType).toBe('Hash Join')
    expect(join.children).toHaveLength(2)
    expect(join.children[1].nodeType).toBe('Hash')
    expect(join.children[1].children).toHaveLength(1)
    expect(join.children[1].children[0].nodeType).toBe('Seq Scan')
    expect(join.children[1].children[0].relation).toBe('orders')
  })

  it('parses Aggregate with Sort', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Aggregate',
          'Strategy': 'Sorted',
          'Startup Cost': 100.00,
          'Total Cost': 100.01,
          'Plan Rows': 1,
          'Plan Width': 8,
          'Plans': [
            {
              'Node Type': 'Sort',
              'Startup Cost': 95.00,
              'Total Cost': 98.00,
              'Plan Rows': 250,
              'Plan Width': 36,
              'Sort Key': ['name ASC'],
              'Plans': [
                {
                  'Node Type': 'Seq Scan',
                  'Relation Name': 'users',
                  'Startup Cost': 0.00,
                  'Total Cost': 35.50,
                  'Plan Rows': 250,
                  'Plan Width': 36,
                },
              ],
            },
          ],
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const agg = nodes[0]
    expect(agg.nodeType).toBe('Aggregate')
    expect(agg.children).toHaveLength(1)
    expect(agg.children[0].nodeType).toBe('Sort')
    expect(agg.children[0].children).toHaveLength(1)
    expect(agg.children[0].children[0].nodeType).toBe('Seq Scan')
  })

  it('parses EXPLAIN ANALYZE with actual rows', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Seq Scan',
          'Relation Name': 'users',
          'Startup Cost': 0.00,
          'Total Cost': 35.50,
          'Plan Rows': 250,
          'Plan Width': 36,
          'Actual Rows': 180,
          'Actual Startup Time': 0.02,
          'Actual Total Time': 0.15,
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.actualRows).toBe(180)
    expect(n.nodeType).toBe('Seq Scan')
  })

  it('parses deeply nested plan (3+ levels)', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Sort',
          'Startup Cost': 500.00,
          'Total Cost': 505.00,
          'Plan Rows': 1000,
          'Plan Width': 48,
          'Plans': [
            {
              'Node Type': 'Hash Join',
              'Startup Cost': 200.00,
              'Total Cost': 450.00,
              'Plan Rows': 1000,
              'Plan Width': 48,
              'Plans': [
                {
                  'Node Type': 'Seq Scan',
                  'Relation Name': 'a',
                  'Startup Cost': 0.00,
                  'Total Cost': 100.00,
                  'Plan Rows': 500,
                  'Plan Width': 24,
                },
                {
                  'Node Type': 'Hash',
                  'Startup Cost': 300.00,
                  'Total Cost': 300.00,
                  'Plan Rows': 500,
                  'Plan Width': 24,
                  'Plans': [
                    {
                      'Node Type': 'Index Only Scan',
                      'Relation Name': 'b',
                      'Index Name': 'idx_b_key',
                      'Startup Cost': 0.00,
                      'Total Cost': 200.00,
                      'Plan Rows': 500,
                      'Plan Width': 24,
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    // Level 1: Sort -> Hash Join -> [Seq Scan, Hash -> Index Only Scan]
    expect(nodes[0].nodeType).toBe('Sort')
    expect(nodes[0].children[0].nodeType).toBe('Hash Join')
    expect(nodes[0].children[0].children[0].nodeType).toBe('Seq Scan')
    expect(nodes[0].children[0].children[1].nodeType).toBe('Hash')
    expect(nodes[0].children[0].children[1].children[0].nodeType).toBe('Index Only Scan')
  })

  it('returns empty array for empty plan', () => {
    const nodes = parsePostgresJson([])
    expect(nodes).toEqual([])
  })

  it('handles plan with missing cost fields', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Result',
          'Plan Rows': 1,
          'Plan Width': 0,
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('Result')
    expect(n.startupCost).toBeUndefined()
    expect(n.totalCost).toBeUndefined()
    expect(n.cost).toBeUndefined()
  })

  it('handles multiple statements gracefully', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Seq Scan',
          'Relation Name': 'a',
          'Startup Cost': 0.00,
          'Total Cost': 10.00,
          'Plan Rows': 100,
          'Plan Width': 4,
        },
      },
      {
        Plan: {
          'Node Type': 'Index Scan',
          'Relation Name': 'b',
          'Startup Cost': 0.00,
          'Total Cost': 5.00,
          'Plan Rows': 50,
          'Plan Width': 8,
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(2)
    expect(nodes[0].nodeType).toBe('Seq Scan')
    expect(nodes[1].nodeType).toBe('Index Scan')
  })

  it('handles Index Only Scan with filter', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Index Only Scan',
          'Relation Name': 'products',
          'Index Name': 'idx_products_category',
          'Startup Cost': 0.10,
          'Total Cost': 25.00,
          'Plan Rows': 80,
          'Plan Width': 16,
          'Index Cond': '(category = 5)',
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('Index Only Scan')
    expect(n.relation).toBe('products')
    expect(n.details).toContain('Index Cond: (category = 5)')
  })

  it('handles Bitmap Heap Scan with Bitmap Index Scan child', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Bitmap Heap Scan',
          'Relation Name': 'logs',
          'Startup Cost': 10.00,
          'Total Cost': 100.00,
          'Plan Rows': 500,
          'Plan Width': 32,
          'Recheck Cond': '(level > 1)',
          'Plans': [
            {
              'Node Type': 'Bitmap Index Scan',
              'Index Name': 'idx_logs_level',
              'Startup Cost': 0.00,
              'Total Cost': 5.00,
              'Plan Rows': 500,
              'Plan Width': 0,
            },
          ],
        },
      },
    ])
    const nodes = parsePostgresJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('Bitmap Heap Scan')
    expect(nodes[0].children).toHaveLength(1)
    expect(nodes[0].children[0].nodeType).toBe('Bitmap Index Scan')
  })
})

// ---------------------------------------------------------------------------
// MySQL JSON parser tests
// ---------------------------------------------------------------------------

describe('parseMySqlJson', () => {
  it('parses a simple table access (ALL)', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'users',
          access_type: 'ALL',
          rows_examined_per_scan: 1000,
          cost_info: {
            query_cost: '105.00',
          },
          used_columns: ['id', 'name', 'email'],
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('ALL')
    expect(n.relation).toBe('users')
    expect(n.rows).toBe(1000)
    expect(n.totalCost).toBe(105.00)
  })

  it('parses a table with WHERE condition and index lookup', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'orders',
          access_type: 'ref',
          possible_keys: ['idx_orders_user'],
          key: 'idx_orders_user',
          rows_examined_per_scan: 5,
          cost_info: {
            query_cost: '2.50',
          },
          used_columns: ['id', 'user_id', 'total'],
          attached_condition: '(orders.user_id = 123)',
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('ref')
    expect(n.relation).toBe('orders')
    expect(n.index).toBe('idx_orders_user')
    expect(n.rows).toBe(5)
    expect(n.details).toContain('(orders.user_id = 123)')
  })

  it('parses a nested loop join', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        nested_loop: [
          {
            table: {
              table_name: 'users',
              access_type: 'ALL',
              rows_examined_per_scan: 100,
              cost_info: { query_cost: '10.00' },
              used_columns: ['id'],
            },
          },
          {
            table: {
              table_name: 'orders',
              access_type: 'ref',
              key: 'idx_orders_user',
              rows_examined_per_scan: 10,
              cost_info: { query_cost: '1.50' },
              used_columns: ['id', 'user_id'],
              attached_condition: '(orders.user_id = users.id)',
            },
          },
        ],
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('Nested Loop Join')
    expect(nodes[0].children).toHaveLength(2)
    expect(nodes[0].children[0].nodeType).toBe('ALL')
    expect(nodes[0].children[0].relation).toBe('users')
    expect(nodes[0].children[1].nodeType).toBe('ref')
    expect(nodes[0].children[1].relation).toBe('orders')
  })

  it('parses a table with ordering_operation', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        ordering_operation: {
          using_filesort: true,
          table: {
            table_name: 'logs',
            access_type: 'ALL',
            rows_examined_per_scan: 5000,
            cost_info: { query_cost: '500.00' },
            used_columns: ['id', 'timestamp'],
          },
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('ALL')
    expect(n.relation).toBe('logs')
    expect(n.details).toContain('Using filesort')
  })

  it('parses a table with grouping_operation', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        grouping_operation: {
          table: {
            table_name: 'sales',
            access_type: 'index',
            key: 'idx_sales_date',
            rows_examined_per_scan: 200,
            cost_info: { query_cost: '30.00' },
            used_columns: ['date', 'amount'],
          },
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('index')
    expect(n.relation).toBe('sales')
    expect(n.index).toBe('idx_sales_date')
  })

  it('parses an eq_ref index lookup', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'customers',
          access_type: 'eq_ref',
          key: 'PRIMARY',
          rows_examined_per_scan: 1,
          cost_info: { query_cost: '1.00' },
          used_columns: ['id', 'name'],
          attached_condition: '(customers.id = 42)',
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('eq_ref')
    expect(nodes[0].index).toBe('PRIMARY')
    expect(nodes[0].rows).toBe(1)
  })

  it('parses a materialized subquery', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'subquery2',
          access_type: 'ALL',
          rows_examined_per_scan: 50,
          cost_info: { query_cost: '5.00' },
          used_columns: ['col1'],
          materialized_from_subquery: {
            query_block: {
              select_id: 2,
              table: {
                table_name: 'raw_data',
                access_type: 'ALL',
                rows_examined_per_scan: 500,
                cost_info: { query_cost: '50.00' },
                used_columns: ['col1'],
              },
            },
          },
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('ALL')
    expect(n.relation).toBe('raw_data')
    expect(n.rows).toBe(500)
  })

  it('returns empty array for empty result', () => {
    const nodes = parseMySqlJson({})
    expect(nodes).toEqual([])
  })

  it('handles const access type', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'constants',
          access_type: 'const',
          rows_examined_per_scan: 1,
          cost_info: { query_cost: '0.50' },
          used_columns: ['value'],
        },
      },
    })
    const nodes = parseMySqlJson(JSON.parse(raw))
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('const')
    expect(nodes[0].relation).toBe('constants')
  })
})

// ---------------------------------------------------------------------------
// SQLite text parser tests
// ---------------------------------------------------------------------------

describe('parseSqliteText', () => {
  it('parses a simple SCAN TABLE', () => {
    const text = `4|0|0|SCAN TABLE users`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('SCAN TABLE')
    expect(n.relation).toBe('users')
  })

  it('parses SEARCH TABLE using index', () => {
    const text = `2|0|0|SEARCH TABLE orders USING INDEX idx_orders_user (user_id=?)`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('SEARCH TABLE')
    expect(n.relation).toBe('orders')
    expect(n.index).toBe('idx_orders_user')
  })

  it('parses a two-table join', () => {
    const text = `1|0|0|SCAN TABLE users
4|1|0|SEARCH TABLE orders USING INDEX idx_orders_user (user_id=?)`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('SCAN TABLE')
    expect(nodes[0].relation).toBe('users')
    expect(nodes[0].children).toHaveLength(1)
    expect(nodes[0].children[0].nodeType).toBe('SEARCH TABLE')
    expect(nodes[0].children[0].relation).toBe('orders')
    expect(nodes[0].children[0].index).toBe('idx_orders_user')
  })

  it('parses a subquery plan', () => {
    const text = `1|0|0|SCAN TABLE users
2|1|1|SCAN TABLE orders
3|0|0|USE TEMP B-TREE FOR ORDER BY`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(2)
    // id=1 is root with id=2 as child, id=3 is separate root
    const btree = nodes.find(n => n.nodeType === 'USE TEMP B-TREE FOR ORDER BY')
    expect(btree).toBeDefined()
  })

  it('parses ORDER BY using temp B-tree', () => {
    const text = `3|0|0|USE TEMP B-TREE FOR ORDER BY`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    const n = nodes[0]
    expect(n.nodeType).toBe('USE TEMP B-TREE FOR ORDER BY')
    expect(n.details[0]).toBe('USE TEMP B-TREE FOR ORDER BY')
  })

  it('returns empty array for empty string', () => {
    const nodes = parseSqliteText('')
    expect(nodes).toEqual([])
  })

  it('parses a multi-level subquery tree', () => {
    const text = `1|0|0|SCAN TABLE parent
2|1|0|SEARCH TABLE child USING INDEX idx_child_parent (parent_id=?)
3|2|0|SEARCH TABLE grandchild USING INDEX idx_grandchild_child (child_id=?)`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('SCAN TABLE')
    expect(nodes[0].relation).toBe('parent')
    expect(nodes[0].children).toHaveLength(1)
    expect(nodes[0].children[0].relation).toBe('child')
    expect(nodes[0].children[0].children).toHaveLength(1)
    expect(nodes[0].children[0].children[0].relation).toBe('grandchild')
  })

  it('parses a CORRELATED SCALAR SUBQUERY', () => {
    const text = `5|0|0|EXECUTE CORRELATED SCALAR SUBQUERY 4`
    const nodes = parseSqliteText(text)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].nodeType).toBe('EXECUTE CORRELATED SCALAR SUBQUERY')
  })
})

// ---------------------------------------------------------------------------
// SQL Server text parser tests
// ---------------------------------------------------------------------------

describe('parseSqlServerText', () => {
  it('parses a simple SELECT', () => {
    const text = `SELECT * FROM users
  |--Table Scan(OBJECT:([users]))`
    const nodes = parseSqlServerText(text)
    expect(nodes.length).toBeGreaterThan(0)
    const tableScan = nodes.find(n => n.nodeType === 'Table Scan')
    expect(tableScan).toBeDefined()
    expect(tableScan!.relation).toBe('users')
  })

  it('parses a JOIN with hash match', () => {
    const text = `SELECT * FROM orders o JOIN customers c ON o.customer_id = c.id
  |--Hash Match(Inner Join, HASH:([o].[customer_id])=([c].[id]))
      |--Table Scan(OBJECT:([orders]))
      |--Table Scan(OBJECT:([customers]))`
    const nodes = parseSqlServerText(text)
    expect(nodes.length).toBeGreaterThan(0)
    const hashMatch = nodes.find(n => n.nodeType === 'Hash Match')
    expect(hashMatch).toBeDefined()
    expect(hashMatch!.details[0]).toContain('Inner Join')
  })

  it('parses an Index Seek', () => {
    const text = `SELECT id FROM products WHERE category = 5
  |--Index Seek(OBJECT:([products].[idx_products_category]), SEEK:([category]=5))`
    const nodes = parseSqlServerText(text)
    expect(nodes.length).toBeGreaterThan(0)
    const idxSeek = nodes.find(n => n.nodeType === 'Index Seek')
    expect(idxSeek).toBeDefined()
    expect(idxSeek!.relation).toBe('products')
    expect(idxSeek!.index).toBe('idx_products_category')
  })

  it('parses Sort + Compute Scalar', () => {
    const text = `SELECT name FROM users ORDER BY name
  |--Sort(ORDER BY:([name] ASC))
      |--Compute Scalar(DEFINE:([name]=[users].[name]))
          |--Table Scan(OBJECT:([users]))`
    const nodes = parseSqlServerText(text)
    expect(nodes.length).toBeGreaterThan(0)
    const sort = nodes.find(n => n.nodeType === 'Sort')
    expect(sort).toBeDefined()
    const compute = nodes.find(n => n.nodeType === 'Compute Scalar')
    expect(compute).toBeDefined()
    const tableScan = nodes.find(n => n.nodeType === 'Table Scan' && n.relation === 'users')
    expect(tableScan).toBeDefined()
  })

  it('parses empty string', () => {
    const nodes = parseSqlServerText('')
    expect(nodes).toEqual([])
  })
})

// ---------------------------------------------------------------------------
// Generic text parser tests (DuckDB / ClickHouse)
// ---------------------------------------------------------------------------

describe('parseGenericText', () => {
  it('parses DuckDB-style explain output', () => {
    const text = `┌─────────────────────────────┐
│         PROJECTION         │
│    ────────────────────    │
│          ~2 Rows           │
│      ──── ──── ────        │
│            #0              │
│          count_star()      │
└─────────────────────────────┘
┌─────────────────────────────┐
│         HASH_GROUP_BY      │
│         ────────────       │
│          Groups: #0        │
│          ~2 Rows           │
├─────────────────────────────┤
│          AGGREGATE         │
│         ────────────       │
│          ~2 Rows           │
└─────────────────────────────┘
┌─────────────────────────────┐
│          SEQ_SCAN           │
│         ────────────        │
│           Table: users      │
│           ~2 Rows           │
└─────────────────────────────┘`
    const nodes = parseGenericText(text)
    // Each non-empty line becomes a node, but we deduplicate by trimming
    expect(nodes.length).toBeGreaterThan(0)
  })

  it('parses ClickHouse-style explain output', () => {
    const text = `EXPLAIN SELECT * FROM users
  Union
  .....
  Expression
  ....
  ReadFromStorage (Read from MergeTree)`
    const nodes = parseGenericText(text)
    expect(nodes.length).toBeGreaterThan(0)
  })

  it('returns empty array for empty string', () => {
    const nodes = parseGenericText('')
    expect(nodes).toEqual([])
  })
})

// ---------------------------------------------------------------------------
// Helper function tests
// ---------------------------------------------------------------------------

describe('getCostColor', () => {
  it('returns green for cost < 10%', () => {
    expect(getCostColor(5, 100)).toBe('green')
    expect(getCostColor(0, 100)).toBe('green')
    expect(getCostColor(9, 100)).toBe('green')
  })

  it('returns yellow for cost between 10% and 40%', () => {
    expect(getCostColor(10, 100)).toBe('yellow')
    expect(getCostColor(25, 100)).toBe('yellow')
    expect(getCostColor(39, 100)).toBe('yellow')
  })

  it('returns red for cost >= 40%', () => {
    expect(getCostColor(40, 100)).toBe('red')
    expect(getCostColor(75, 100)).toBe('red')
    expect(getCostColor(100, 100)).toBe('red')
  })

  it('returns green when totalCost <= 0', () => {
    expect(getCostColor(99, 0)).toBe('green')
    expect(getCostColor(99, -1)).toBe('green')
  })
})

describe('findMostExpensiveNode', () => {
  it('finds the node with highest totalCost', () => {
    const nodes: ExplainPlanNode[] = [
      {
        id: '1',
        title: 'Seq Scan',
        nodeType: 'Seq Scan',
        details: [],
        children: [],
        totalCost: 10,
      },
      {
        id: '2',
        title: 'Index Scan',
        nodeType: 'Index Scan',
        details: [],
        children: [],
        totalCost: 50,
      },
      {
        id: '3',
        title: 'Sort',
        nodeType: 'Sort',
        details: [],
        children: [],
        totalCost: 25,
      },
    ]
    expect(findMostExpensiveNode(nodes)).toBe('Index Scan')
  })

  it('returns undefined for empty array', () => {
    expect(findMostExpensiveNode([])).toBeUndefined()
  })

  it('ignores nodes without totalCost', () => {
    const nodes: ExplainPlanNode[] = [
      { id: '1', title: 'Result', nodeType: 'Result', details: [], children: [] },
    ]
    expect(findMostExpensiveNode(nodes)).toBeUndefined()
  })
})

describe('calculateTotalCost', () => {
  it('sums totalCost of all root nodes', () => {
    const nodes: ExplainPlanNode[] = [
      { id: '1', title: 'A', nodeType: 'A', totalCost: 10, details: [], children: [] },
      { id: '2', title: 'B', nodeType: 'B', totalCost: 20, details: [], children: [] },
      { id: '3', title: 'C', nodeType: 'C', totalCost: 30, details: [], children: [] },
    ]
    expect(calculateTotalCost(nodes)).toBe(60)
  })

  it('returns 0 for empty array', () => {
    expect(calculateTotalCost([])).toBe(0)
  })

  it('skips nodes without totalCost', () => {
    const nodes: ExplainPlanNode[] = [
      { id: '1', title: 'A', nodeType: 'A', totalCost: 10, details: [], children: [] },
      { id: '2', title: 'B', nodeType: 'B', details: [], children: [] },
    ]
    expect(calculateTotalCost(nodes)).toBe(10)
  })
})

describe('flattenNodes', () => {
  it('flattens a tree structure into an array', () => {
    const leaf1: ExplainPlanNode = { id: '1.0', title: 'Leaf1', nodeType: 'Scan', details: [], children: [] }
    const leaf2: ExplainPlanNode = { id: '1.1', title: 'Leaf2', nodeType: 'Index', details: [], children: [] }
    const parent: ExplainPlanNode = {
      id: '1',
      title: 'Join',
      nodeType: 'Join',
      details: [],
      children: [leaf1, leaf2],
    }
    const flat = flattenNodes([parent])
    expect(flat).toHaveLength(3)
    expect(flat[0].id).toBe('1')
    expect(flat[1].id).toBe('1.0')
    expect(flat[2].id).toBe('1.1')
  })

  it('handles empty array', () => {
    expect(flattenNodes([])).toEqual([])
  })

  it('flattens deeply nested trees', () => {
    const leaf: ExplainPlanNode = { id: '0.0.0', title: 'Deep Leaf', nodeType: 'Scan', details: [], children: [] }
    const mid: ExplainPlanNode = { id: '0.0', title: 'Mid', nodeType: 'Join', details: [], children: [leaf] }
    const root: ExplainPlanNode = { id: '0', title: 'Root', nodeType: 'Sort', details: [], children: [mid] }
    const flat = flattenNodes([root])
    expect(flat).toHaveLength(3)
  })
})

// ---------------------------------------------------------------------------
// parseExplainResult integration tests
// ---------------------------------------------------------------------------

describe('parseExplainResult', () => {
  it('dispatches to PostgreSQL JSON parser when databaseType is postgresql and format is json', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Seq Scan',
          'Relation Name': 'users',
          'Startup Cost': 0.00,
          'Total Cost': 35.50,
          'Plan Rows': 250,
          'Plan Width': 36,
        },
      },
    ])
    const result = parseExplainResult(raw, 'postgresql', 'json', false)
    expect(result.databaseType).toBe('postgresql')
    expect(result.format).toBe('json')
    expect(result.nodes).toHaveLength(1)
    expect(result.nodes[0].nodeType).toBe('Seq Scan')
  })

  it('dispatches to MySQL JSON parser when databaseType is mysql and format is json', () => {
    const raw = JSON.stringify({
      query_block: {
        select_id: 1,
        table: {
          table_name: 'users',
          access_type: 'ALL',
          rows_examined_per_scan: 100,
          cost_info: { query_cost: '10.00' },
          used_columns: ['id'],
        },
      },
    })
    const result = parseExplainResult(raw, 'mysql', 'json', false)
    expect(result.databaseType).toBe('mysql')
    expect(result.nodes).toHaveLength(1)
    expect(result.nodes[0].nodeType).toBe('ALL')
  })

  it('dispatches to SQLite text parser when databaseType is sqlite', () => {
    const raw = `4|0|0|SCAN TABLE users`
    const result = parseExplainResult(raw, 'sqlite', 'text', false)
    expect(result.databaseType).toBe('sqlite')
    expect(result.nodes).toHaveLength(1)
    expect(result.nodes[0].nodeType).toBe('SCAN TABLE')
    expect(result.totalCost).toBe(0)
    expect(result.mostExpensiveNode).toBeUndefined()
  })

  it('dispatches to SQL Server text parser when databaseType is sqlserver', () => {
    const raw = `SELECT * FROM users
  |--Table Scan(OBJECT:([users]))`
    const result = parseExplainResult(raw, 'sqlserver', 'text', false)
    expect(result.databaseType).toBe('sqlserver')
    expect(result.nodes.length).toBeGreaterThan(0)
  })

  it('dispatches to generic text parser for DuckDB', () => {
    const raw = `┌─────────────┐
│  SEQ_SCAN   │
└─────────────┘`
    const result = parseExplainResult(raw, 'duckdb', 'text', false)
    expect(result.databaseType).toBe('duckdb')
    expect(result.nodes.length).toBeGreaterThan(0)
  })

  it('dispatches to generic text parser for ClickHouse', () => {
    const raw = `Expression
ReadFromStorage`
    const result = parseExplainResult(raw, 'clickhouse', 'text', false)
    expect(result.databaseType).toBe('clickhouse')
    expect(result.nodes.length).toBeGreaterThan(0)
  })

  it('sets analyze flag correctly', () => {
    const raw = JSON.stringify([
      {
        Plan: {
          'Node Type': 'Seq Scan',
          'Relation Name': 'users',
          'Startup Cost': 0.00,
          'Total Cost': 35.50,
          'Plan Rows': 250,
          'Plan Width': 36,
        },
      },
    ])
    const result = parseExplainResult(raw, 'postgresql', 'json', true)
    expect(result.analyze).toBe(true)
  })

  it('preserves raw output', () => {
    const raw = `4|0|0|SCAN TABLE users`
    const result = parseExplainResult(raw, 'sqlite', 'text', false)
    expect(result.raw).toBe(raw)
  })

  describe('parseMySqlAnalyzeText', () => {
    it('parses a simple table scan', () => {
      const text = `-> Table scan on users  (cost=0.5 rows=2)`
      const nodes = parseMySqlAnalyzeText(text)
      expect(nodes).toHaveLength(1)
      expect(nodes[0].nodeType).toBe('Table scan')
      expect(nodes[0].relation).toBe('users')
      expect(nodes[0].totalCost).toBe(0.5)
      expect(nodes[0].rows).toBe(2)
    })

    it('parses a nested loop join with two children', () => {
      const text = [
        '-> Nested loop inner join  (cost=1.0 rows=1)',
        '    -> Table scan on users  (cost=0.5 rows=2)',
        '    -> Index lookup on orders using user_id  (cost=0.5 rows=1)',
      ].join('\n')
      const nodes = parseMySqlAnalyzeText(text)
      expect(nodes).toHaveLength(1)
      expect(nodes[0].nodeType).toBe('Nested loop inner join')
      expect(nodes[0].children).toHaveLength(2)
      expect(nodes[0].children[0].nodeType).toBe('Table scan')
      expect(nodes[0].children[0].relation).toBe('users')
      expect(nodes[0].children[1].relation).toBe('orders')
    })

    it('parses deeply nested plans', () => {
      const text = [
        '-> Nested loop inner join  (cost=2.0 rows=1)',
        '    -> Table scan on users  (cost=0.5 rows=2)',
        '    -> Nested loop inner join  (cost=1.5 rows=1)',
        '        -> Index lookup on orders  (cost=0.5 rows=1)',
        '        -> Index lookup on payments  (cost=0.5 rows=1)',
      ].join('\n')
      const nodes = parseMySqlAnalyzeText(text)
      expect(nodes).toHaveLength(1)
      expect(nodes[0].children).toHaveLength(2)
      expect(nodes[0].children[1].nodeType).toBe('Nested loop inner join')
      expect(nodes[0].children[1].children).toHaveLength(2)
    })

    it('returns empty array for empty input', () => {
      expect(parseMySqlAnalyzeText('')).toEqual([])
    })

    it('returns empty array when no -> lines', () => {
      expect(parseMySqlAnalyzeText('some random text')).toEqual([])
    })
  })
})
