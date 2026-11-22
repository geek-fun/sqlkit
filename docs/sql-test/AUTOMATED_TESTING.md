# Live SQLKit regression tests

Run all four fixtures through SQLKit's real Rust adapters, including Oracle's
Java JDBC bridge, against the existing `event-search` SSH test stack:

```bash
python3 scripts/test-sql-stress.py
```

Prerequisites: the four running database containers and SSH bastion from
`event-search/docker-compose-sqlkit-ssh-tunnel.yml`, Rust, Python 3, Maven, and a
JDK 25+ for rebuilding the bridge. If Maven cannot find your JDK, pass
`--java-home /absolute/path/to/jdk`. Oracle's managed JRE and cached JDBC driver
are used by SQLKit as usual; missing runtime dependencies may be downloaded.

The runner finds the test key in the sibling `event-search` checkout, opens
loopback SSH forwards on available ports, rebuilds the local bridge, and runs
`src-tauri/tests/sql_stress_integration.rs`. It closes its SSH process on exit.
The local bridge is selected with `SQLKIT_JDBC_BRIDGE_JAR`; the runner does not
replace the installed bridge JAR.

**This recreates the objects owned by `01_schema.sql` and reloads the seed data.**
Use disposable test databases. The scripts retain their documented effects,
including PostgreSQL's public fixture types, MySQL's three fixture databases,
and SQL Server's `testdb` objects. They do not reset Docker volumes.

Useful options:

```bash
python3 scripts/test-sql-stress.py --engine postgres
python3 scripts/test-sql-stress.py --ssh-key /absolute/path/to/test_key
python3 scripts/test-sql-stress.py --skip-bridge-build
python3 scripts/test-sql-stress.py --direct
```

For existing forwards or directly exposed databases, `--direct` uses the test
connection defaults below. Override them with `{PREFIX}_HOST`, `{PREFIX}_PORT`,
`{PREFIX}_USER`, `{PREFIX}_PASSWORD`, and `{PREFIX}_DB` as appropriate. Fixture SQL
still names its documented databases and schemas explicitly.

| Prefix | Host | Port | User | Database/service |
| --- | --- | --- | --- | --- |
| POSTGRES | 127.0.0.1 | 25432 | tester | testdb |
| MYSQL | 127.0.0.1 | 23306 | root | sqlkit-demo |
| SQLSERVER | 127.0.0.1 | 21433 | sa | testdb |
| ORACLE | 127.0.0.1 | 21521 | sqlkit | FREEPDB1 |

An already-loaded fixture can also be tested without schema/seed recreation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test sql_stress_integration -- --ignored --nocapture
```

Set `SQLKIT_STRESS_RESET=1` to recreate it. Reload the fixture before repeating
query tests: some query sections intentionally mutate data.

## Coverage

- All four `01_schema.sql` and `02_seed.sql` files execute through SQLKit's
  adapters. The harness removes MySQL client `DELIMITER` commands and splits
  SQL Server `GO` batches, as those are client directives. PostgreSQL and Oracle
  scripts are sent directly to their adapters, including procedural definitions.
- Every banner-delimited section in each `03_complex_queries.sql` executes.
  PostgreSQL's expected exclusion violation is checked by SQLSTATE `23P01`,
  then its savepoint recovery executes. SQL Server's expected overlap error is
  accepted only in section I; an outstanding transaction is rolled back before
  proceeding. Errors swallowed by the fixture's stored procedures stay internal.
- Seed counts: 400 users, 60 products, 500 orders, 1,250 order items per engine.
- Schemas, views, routines, and every table's columns, table information,
  indexes, foreign keys, and triggers; column names must be unique.
- DDL retrieval for views and Oracle tables; native table DDL uses SQLKit's
  existing transfer generator. This checks availability, not full DDL round trips.
- Rename/drop on dedicated scratch tables; empty-result column metadata;
  full user-row conversion, numeric precision/scale, PostgreSQL identity columns,
  GIN/BRIN/primary indexes, and ordered composite foreign keys.
- PostgreSQL multi-statement transactions, composite/network/geometric values,
  arbitrary-precision numerics and unsupported array element types.
- MySQL session variables across requests and metadata access, later result sets,
  affected-row aggregation, later-statement errors, unsigned integers and binary data.
- SQL Server session/temp-table persistence across batches, affected rows for
  individual DML statements, and final/empty result sets.
- Java unit regressions cover Oracle JSON, vendor timestamps, object types,
  nulls and booleans. Frontend tests cover cursor extraction around quoted
  semicolons, comments, PostgreSQL dollar-quoted bodies and Windows line endings.

These are adapter and parser checks. They do not validate rendered desktop panels,
keyboard interactions, visual layout, or complete DDL reconstruction. Oracle still
executes separate JDBC requests in autocommit mode; its transaction fixture uses
stored procedures to keep related work in one request. SQL Server `NOCOUNT ON`
suppresses affected-row counts; the explicit row-count probe turns it off.

## Verification

On 2026-10-17, the existing SSH stack passed all four live suites: PostgreSQL 20 query sections,
MySQL 24, SQL Server 20, and Oracle 20 (84 sections total), after recreating and
seeding all fixtures through SQLKit. The full Rust test command passed 547 tests
with live tests ignored by default; the explicit live run passed all four suites.
On 2026-10-18, the standalone fix passed 668 frontend tests. Java: 10 tests passed. Frontend production build passed;
ESLint reported zero errors and one existing UnoCSS ordering warning in
`DefaultAvatar.vue`.

Bugs fixed during these runs:

- PostgreSQL: multi-statement execution; unsafe binary rendering of exotic types
  and large numerics; incorrect index definitions/types/flags; duplicate or
  misordered composite foreign-key columns; missing identity/generated metadata;
  negative estimated row counts wrapping into huge unsigned counts.
- MySQL: functional-index metadata crash on NULL column names; lost session state;
  discarded procedure/later result sets and late batch errors; incorrect numeric,
  unsigned and binary values; affected-row aggregation; unsupported table drops.
- SQL Server: new physical connections per request losing temp tables and SET
  options; discarded final/empty result sets; missing individual DML row counts;
  duplicate column metadata and missing precision/scale/character lengths.
- Oracle/JDBC: native JSON and vendor timestamp conversion failures; object-type
  rendering crashing the bridge; incomplete routine DDL and missing table DDL;
  incorrect numeric precision; PL/SQL bodies split at internal semicolons;
  comment removal merging tokens/skipping the next character; booleans and
  structured values losing their type/content in the Rust response conversion.
- Editor: semicolons inside strings, identifiers, comments and dollar-quoted
  PostgreSQL bodies incorrectly selecting fragments of the statement.
