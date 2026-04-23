//! Best-practice default configurations for transfer formats.

use super::types::*;

pub fn csv_export_defaults() -> CsvExportOptions {
    CsvExportOptions {
        delimiter: ',',
        quote_char: '"',
        encoding: "UTF-8".to_string(),
        include_header: true,
        quote_all: false,
        line_ending: "LF".to_string(),
    }
}

pub fn jsonl_export_defaults() -> JsonlExportOptions {
    JsonlExportOptions {
        date_format: "ISO8601".to_string(),
    }
}

pub fn sql_export_defaults(table_name: &str) -> SqlExportOptions {
    SqlExportOptions {
        target_table: table_name.to_string(),
        batch_size: 1000,
        include_create_table: true,
        include_drop_table: false,
        target_engine: None,
    }
}

pub fn excel_export_defaults() -> ExcelExportOptions {
    ExcelExportOptions {
        sheet_name: "Sheet1".to_string(),
        include_header: true,
        auto_fit_columns: true,
        freeze_header: true,
    }
}

pub fn csv_import_defaults() -> CsvImportOptions {
    CsvImportOptions {
        delimiter: ',',
        encoding: "UTF-8".to_string(),
        has_header: true,
    }
}
