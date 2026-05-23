//! Transfer module for data export, import, and migration.

use crate::database::DatabaseType;

pub mod ddl;
pub mod defaults;
pub mod export;
pub mod import;
pub mod migration;
pub mod profile_store;
pub mod progress;
pub mod restore;
pub mod types;

pub use ddl::*;
pub use defaults::*;
pub use export::*;
pub use import::*;
pub use migration::*;
pub use profile_store::*;
pub use progress::*;
pub use restore::*;
pub use types::*;

/// Returns the pagination clause for a given DB type.
///
/// `base_has_order_by` must be `true` if the base SELECT already has an `ORDER BY`
/// clause appended. For SQL Server this avoids emitting a second `ORDER BY (SELECT NULL)`
/// (which would produce invalid T-SQL). For other engines it is informational only.
pub fn paginate_clause(
    db_type: DatabaseType,
    offset: usize,
    limit: usize,
    base_has_order_by: bool,
) -> String {
    match db_type {
        DatabaseType::SqlServer => {
            if base_has_order_by {
                format!("OFFSET {} ROWS FETCH NEXT {} ROWS ONLY", offset, limit)
            } else {
                format!(
                    "ORDER BY (SELECT NULL) OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                    offset, limit
                )
            }
        }
        _ => format!("LIMIT {} OFFSET {}", limit, offset),
    }
}

#[cfg(test)]
mod tests {
    use super::paginate_clause;
    use crate::database::DatabaseType;

    #[test]
    fn paginate_clause_uses_limit_offset_for_postgres() {
        assert_eq!(
            paginate_clause(DatabaseType::PostgreSQL, 10, 25, false),
            "LIMIT 25 OFFSET 10"
        );
    }

    #[test]
    fn paginate_clause_uses_limit_offset_for_mysql() {
        assert_eq!(
            paginate_clause(DatabaseType::MySQL, 10, 25, false),
            "LIMIT 25 OFFSET 10"
        );
    }

    #[test]
    fn paginate_clause_uses_limit_offset_for_sqlite() {
        assert_eq!(
            paginate_clause(DatabaseType::SQLite, 10, 25, false),
            "LIMIT 25 OFFSET 10"
        );
    }

    #[test]
    fn paginate_clause_uses_offset_fetch_for_sqlserver_without_order_by() {
        assert_eq!(
            paginate_clause(DatabaseType::SqlServer, 10, 25, false),
            "ORDER BY (SELECT NULL) OFFSET 10 ROWS FETCH NEXT 25 ROWS ONLY"
        );
    }

    #[test]
    fn paginate_clause_skips_synthetic_order_by_when_base_query_has_one() {
        assert_eq!(
            paginate_clause(DatabaseType::SqlServer, 10, 25, true),
            "OFFSET 10 ROWS FETCH NEXT 25 ROWS ONLY"
        );
    }

    #[test]
    fn paginate_clause_ignores_base_order_by_flag_for_non_sqlserver() {
        assert_eq!(
            paginate_clause(DatabaseType::PostgreSQL, 10, 25, true),
            "LIMIT 25 OFFSET 10"
        );
    }
}
