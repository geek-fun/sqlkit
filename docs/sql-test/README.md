# SQLKit PostgreSQL Stress-Test Suite

Realistic e-commerce business data (users, merchants, products, orders,
payments, reviews, coupons) designed to exercise every SQLKit feature panel
against a complex PostgreSQL setup.

**Target:** the docker ssh-tunnel test stack — `pg-test:5432/testdb`
(`tester` / `testpass`), reachable through the bastion at `localhost:2223`.

Execute the files **in order**:

| File | What it does |
|------|--------------|
| `01_schema.sql` | Creates 3 schemas (`store` / `analytics` / `billing`), enums + domains + composite types, 12+ tables (partitioned orders, composite PK/FK, generated column, EXCLUDE constraints, deferrable unique), views (updatable / `WITH CHECK OPTION` / security barrier), a materialized view, functions (overloaded, cross-schema same name), procedures, and row/statement-level triggers. |
| `02_seed.sql` | Generates deterministic business data: 400 users, 10 merchants, 60 products, 500 orders across the Q1 partitions, ~1,200 line items, payments, reviews, coupons, audit rows. Ends with a per-table count card so you can verify the load. |
| `03_complex_queries.sql` | A 19-section query playground: multi-CTE reports, `WITH RECURSIVE` category tree, `LATERAL` top-N, window functions (moving average / lag / lead / RANGE frames), `GROUPING SETS` + `FILTER`, `FULL OUTER JOIN` / `INTERSECT` / `EXCEPT`, `DISTINCT ON`, `ON CONFLICT` upserts with `RETURNING`, `MERGE`, transactions with `SAVEPOINT` rollback, `DO` blocks, `CALL` procedures, `EXPLAIN (ANALYZE, BUFFERS)` + partition pruning, `UPDATE FROM` / `DELETE USING`, deferred-constraint swaps. |

Each section in `03_complex_queries.sql` is separated by banner comments —
run them one at a time (or select-and-execute) to test the query editor,
result grid, large result sets, multi-statement execution, and error paths.

## Suggested SQLKit checks

- **Schemas panel** — 3 schemas should list and switch.
- **Tables / Columns / Table Info** — partitioned parent vs leaf partitions,
  composite PKs, generated columns, `COMMENT ON` metadata.
- **Indexes panel** — expression / partial / trigram GIN / BRIN / covering indexes.
- **Foreign Keys panel** — named & unnamed (default) constraints, composite FK,
  `ON DELETE CASCADE / SET NULL / RESTRICT`.
- **Triggers / Views / Procedures / Functions panels** — including the
  same-name functions in `store` vs `analytics` schemas (schema-scoping check).
- **DDL preview** on tables, views, materialized views, partitioned tables.
- **Drop / Rename** on throwaway objects (e.g. the temp-style tables).
- **Query editor** — run `03_complex_queries.sql` section 1..19, each should
  execute / produce a meaningful result grid / clean error.

## Notes

- Everything is idempotent: `01_schema.sql` drops and recreates all objects;
  `02_seed.sql` truncates first. Re-running the suite resets to a fresh state.
- `03_complex_queries.sql` contains exactly one expected error: section I
  intentionally violates the `reservations_no_overlap` EXCLUDE constraint and
  immediately rescues the transaction with `ROLLBACK TO SAVEPOINT`. Any other
  error means something needs fixing (run with `ON_ERROR_STOP` when debugging).
- `adminpack` may not be available on newer PostgreSQL builds — if the
  `CREATE EXTENSION adminpack` line errors, delete just that line; the rest
  of the suite does not depend on it.
- The `tester` user in the docker stack is a superuser, so extension creation
  works. On a managed cluster you may need `pg_trgm` / `btree_gist` privileges
  or DBA cooperation.
- Trigger-driven audit rows accumulate in `store.audit_log` — its content is
  a good way to verify that SQLKit-issued INSERT/UPDATE/DELETE statements
  really hit the database behind the tunnel.
