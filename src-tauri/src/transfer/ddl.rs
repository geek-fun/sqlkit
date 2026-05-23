//! DDL generation for various database engines.

use crate::database::types::ColumnInfo;
use crate::database::DatabaseType;

use super::types::*;

pub fn generate_ddl_for_engine(
    engine: DatabaseType,
    schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    match engine {
        DatabaseType::PostgreSQL => generate_postgres_ddl(schema, table, columns, options),
        DatabaseType::MySQL => generate_mysql_ddl(schema, table, columns, options),
        DatabaseType::SQLite => generate_sqlite_ddl(schema, table, columns, options),
        DatabaseType::SqlServer => generate_sqlserver_ddl(schema, table, columns, options),
        _ => generate_generic_ddl(schema, table, columns, options),
    }
}

fn generate_postgres_ddl(
    schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    let mut sql = String::new();
    let table_ref = format_table_ref_postgres(schema, table);

    if options.include_drop_if_exists {
        sql.push_str(&format!("DROP TABLE IF EXISTS {} CASCADE;\n", table_ref));
    }

    if options.include_create_table {
        let create_keyword = if options.include_if_not_exists {
            "CREATE TABLE IF NOT EXISTS"
        } else {
            "CREATE TABLE"
        };

        sql.push_str(&format!("{} {} (\n", create_keyword, table_ref));

        let col_defs: Vec<String> = columns
            .iter()
            .map(|c| format_column_postgres(c, options))
            .collect();

        let pk_cols: Vec<String> = columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| quote_identifier_postgres(&c.name))
            .collect();

        if !pk_cols.is_empty() && options.include_primary_keys {
            let pk_def = format!("  PRIMARY KEY ({})", pk_cols.join(", "));
            sql.push_str(&col_defs.join(",\n"));
            sql.push_str(",\n");
            sql.push_str(&pk_def);
        } else {
            sql.push_str(&col_defs.join(",\n"));
        }

        sql.push_str("\n);\n");
    }

    if options.include_comments {
        for col in columns {
            if let Some(ref desc) = col.description {
                if !desc.is_empty() {
                    sql.push_str(&format!(
                        "COMMENT ON COLUMN {}.{} IS '{}';\n",
                        table_ref,
                        quote_identifier_postgres(&col.name),
                        desc.replace('\'', "''")
                    ));
                }
            }
        }
    }

    sql
}

fn format_column_postgres(col: &ColumnInfo, options: &DdlOptions) -> String {
    let name = quote_identifier_postgres(&col.name);
    let mut def = format!("  {} {}", name, col.data_type);

    if !col.nullable && !col.is_primary_key {
        def.push_str(" NOT NULL");
    }

    if col.is_primary_key
        && options.include_primary_keys
        && col.is_auto_increment
        && col.data_type.to_lowercase().contains("int")
    {
        def.push_str(" PRIMARY KEY");
    }

    if let Some(ref default) = col.default_value {
        if !col.is_auto_increment {
            def.push_str(&format!(" DEFAULT {}", default));
        }
    }

    def
}

fn quote_identifier_postgres(name: &str) -> String {
    format!("\"{}\"", name)
}

fn format_table_ref_postgres(schema: Option<&str>, table: &str) -> String {
    match schema {
        Some(s) => format!("\"{}\".\"{}\"", s, table),
        None => format!("\"{}\"", table),
    }
}

fn generate_mysql_ddl(
    schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    let mut sql = String::new();
    let table_ref = format_table_ref_mysql(schema, table);

    if options.include_drop_if_exists {
        sql.push_str(&format!("DROP TABLE IF EXISTS {};\n", table_ref));
    }

    if options.include_create_table {
        let create_keyword = if options.include_if_not_exists {
            "CREATE TABLE IF NOT EXISTS"
        } else {
            "CREATE TABLE"
        };

        sql.push_str(&format!("{} {} (\n", create_keyword, table_ref));

        let col_defs: Vec<String> = columns.iter().map(format_column_mysql).collect();

        let pk_cols: Vec<String> = columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| quote_identifier_mysql(&c.name))
            .collect();

        if !pk_cols.is_empty() && options.include_primary_keys {
            let pk_def = format!("  PRIMARY KEY ({})", pk_cols.join(", "));
            sql.push_str(&col_defs.join(",\n"));
            sql.push_str(",\n");
            sql.push_str(&pk_def);
        } else {
            sql.push_str(&col_defs.join(",\n"));
        }

        sql.push_str("\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;\n");
    }

    if options.include_comments {
        for col in columns {
            if let Some(ref desc) = col.description {
                if !desc.is_empty() {
                    sql.push_str(&format!(
                        "ALTER TABLE {} MODIFY COLUMN {} {} COMMENT '{}';\n",
                        table_ref,
                        quote_identifier_mysql(&col.name),
                        col.data_type,
                        desc.replace('\'', "''")
                    ));
                }
            }
        }
    }

    sql
}

fn format_column_mysql(col: &ColumnInfo) -> String {
    let name = quote_identifier_mysql(&col.name);
    let mut def = format!("  {} {}", name, col.data_type);

    if col.is_auto_increment {
        def.push_str(" AUTO_INCREMENT");
    }

    if !col.nullable && !col.is_primary_key {
        def.push_str(" NOT NULL");
    }

    if let Some(ref default) = col.default_value {
        if !col.is_auto_increment {
            def.push_str(&format!(" DEFAULT {}", default));
        }
    }

    def
}

fn quote_identifier_mysql(name: &str) -> String {
    format!("`{}`", name)
}

fn format_table_ref_mysql(schema: Option<&str>, table: &str) -> String {
    match schema {
        Some(s) => format!("`{}`.`{}`", s, table),
        None => format!("`{}`", table),
    }
}

fn generate_sqlite_ddl(
    _schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    let mut sql = String::new();

    if options.include_drop_if_exists {
        sql.push_str(&format!("DROP TABLE IF EXISTS \"{}\";\n", table));
    }

    if options.include_create_table {
        let create_keyword = if options.include_if_not_exists {
            "CREATE TABLE IF NOT EXISTS"
        } else {
            "CREATE TABLE"
        };

        sql.push_str(&format!("{} \"{}\" (\n", create_keyword, table));

        let col_defs: Vec<String> = columns.iter().map(format_column_sqlite).collect();

        let pk_cols: Vec<String> = columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| format!("\"{}\"", c.name))
            .collect();

        if !pk_cols.is_empty() && options.include_primary_keys {
            let pk_def = format!("  PRIMARY KEY ({})", pk_cols.join(", "));
            sql.push_str(&col_defs.join(",\n"));
            sql.push_str(",\n");
            sql.push_str(&pk_def);
        } else {
            sql.push_str(&col_defs.join(",\n"));
        }

        sql.push_str("\n);\n");
    }

    sql
}

fn format_column_sqlite(col: &ColumnInfo) -> String {
    let name = format!("\"{}\"", col.name);
    let mut def = format!("  {} {}", name, col.data_type);

    if col.is_primary_key && col.is_auto_increment {
        def.push_str(" PRIMARY KEY AUTOINCREMENT");
    } else {
        if !col.nullable && !col.is_primary_key {
            def.push_str(" NOT NULL");
        }

        if let Some(ref default) = col.default_value {
            def.push_str(&format!(" DEFAULT {}", default));
        }
    }

    def
}

fn generate_sqlserver_ddl(
    schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    let mut sql = String::new();
    let table_ref = format_table_ref_sqlserver(schema, table);

    if options.include_drop_if_exists {
        sql.push_str(&format!(
            "IF OBJECT_ID('{}', 'U') IS NOT NULL DROP TABLE {};\n",
            table_ref, table_ref
        ));
    }

    if options.include_create_table {
        sql.push_str(&format!("CREATE TABLE {} (\n", table_ref));

        let col_defs: Vec<String> = columns.iter().map(format_column_sqlserver).collect();

        let pk_cols: Vec<String> = columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| quote_identifier_sqlserver(&c.name))
            .collect();

        if !pk_cols.is_empty() && options.include_primary_keys {
            let pk_def = format!("  PRIMARY KEY ({})", pk_cols.join(", "));
            sql.push_str(&col_defs.join(",\n"));
            sql.push_str(",\n");
            sql.push_str(&pk_def);
        } else {
            sql.push_str(&col_defs.join(",\n"));
        }

        sql.push_str("\n);\n");
    }

    sql
}

fn format_column_sqlserver(col: &ColumnInfo) -> String {
    let name = quote_identifier_sqlserver(&col.name);
    let mut def = format!("  {} {}", name, col.data_type);

    if col.is_auto_increment {
        def.push_str(" IDENTITY(1,1)");
    }

    if !col.nullable && !col.is_primary_key {
        def.push_str(" NOT NULL");
    }

    if let Some(ref default) = col.default_value {
        if !col.is_auto_increment {
            def.push_str(&format!(" DEFAULT {}", default));
        }
    }

    def
}

fn quote_identifier_sqlserver(name: &str) -> String {
    format!("[{}]", name)
}

fn format_table_ref_sqlserver(schema: Option<&str>, table: &str) -> String {
    match schema {
        Some(s) => format!("[{}].[{}]", s, table),
        None => format!("[{}]", table),
    }
}

fn generate_generic_ddl(
    schema: Option<&str>,
    table: &str,
    columns: &[ColumnInfo],
    options: &DdlOptions,
) -> String {
    generate_postgres_ddl(schema, table, columns, options)
}
