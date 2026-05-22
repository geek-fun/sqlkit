//! Transfer feature type definitions.
//!
//! This module defines all types for data export, import, and migration operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Export Types ────────────────────────────────────────────────

/// Supported export formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Csv,
    Jsonl,
    Sql,
    Excel,
}

/// CSV export options with sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportOptions {
    #[serde(default = "default_delimiter")]
    pub delimiter: char,
    #[serde(default = "default_quote_char")]
    pub quote_char: char,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default = "default_true")]
    pub include_header: bool,
    #[serde(default)]
    pub quote_all: bool,
    #[serde(default = "default_lf")]
    pub line_ending: String,
}

/// JSONL (JSON Lines) export options.
/// Simpler than JSON: one object per line, compact format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonlExportOptions {
    #[serde(default = "default_iso8601")]
    pub date_format: String,
}

/// SQL export options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlExportOptions {
    pub target_table: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
    #[serde(default = "default_true")]
    pub include_create_table: bool,
    #[serde(default)]
    pub include_drop_table: bool,
    pub target_engine: Option<String>,
}

/// Excel export options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelExportOptions {
    #[serde(default = "default_sheet_name")]
    pub sheet_name: String,
    #[serde(default = "default_true")]
    pub include_header: bool,
    #[serde(default = "default_true")]
    pub auto_fit_columns: bool,
    #[serde(default = "default_true")]
    pub freeze_header: bool,
}

/// Export source is always a table (Custom Query removed for simplicity).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSource {
    pub table: String,
    pub columns: Vec<String>,
    pub where_clause: Option<String>,
    pub order_by: Option<String>,
    pub limit: Option<u64>,
}

/// Export request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub source: ExportSource,
    pub format: ExportFormat,
    pub csv_options: Option<CsvExportOptions>,
    pub jsonl_options: Option<JsonlExportOptions>,
    pub sql_options: Option<SqlExportOptions>,
    pub excel_options: Option<ExcelExportOptions>,
    pub output_path: String,
}

// ── Import Types ────────────────────────────────────────────────

/// Supported import formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ImportFormat {
    Csv,
    Jsonl,
    Sql,
    Excel,
}

/// Column mapping for import.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source_column: String,
    pub target_column: Option<String>, // None = skip
    pub target_type: Option<String>,
}

/// Conflict resolution strategy for import.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConflictStrategy {
    #[default]
    Skip,
    Replace,
    Upsert,
    Abort,
}

/// CSV import options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportOptions {
    #[serde(default = "default_delimiter")]
    pub delimiter: char,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default = "default_true")]
    pub has_header: bool,
}

/// Excel import options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelImportOptions {
    #[serde(default = "default_sheet_name")]
    pub sheet_name: String,
    #[serde(default = "default_true")]
    pub has_header: bool,
}

/// Import request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub table: String,
    pub file_path: String,
    pub format: ImportFormat,
    pub column_mappings: Vec<ColumnMapping>,
    #[serde(default)]
    pub conflict_strategy: ConflictStrategy,
    #[serde(default = "default_import_batch_size")]
    pub batch_size: u32,
    #[serde(default)]
    pub create_table: bool,
    #[serde(default)]
    pub truncate_before: bool,
    #[serde(default)]
    pub dry_run: bool,
    pub csv_options: Option<CsvImportOptions>,
    pub excel_options: Option<ExcelImportOptions>,
}

// ── Progress & Results ──────────────────────────────────────────

/// Transfer operation progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub operation: String, // "export" | "import" | "ddl" | "sql_file" | "migration"
    pub phase: String,     // "preparing" | "processing" | "finalizing"
    pub current_table: Option<String>,
    pub total_rows: Option<u64>,
    pub processed_rows: u64,
    pub skipped_rows: u64,
    pub error_count: u64,
    pub percent: f32, // 0.0–100.0
    pub elapsed_ms: u64,
    pub estimated_remaining_ms: Option<u64>,
    pub message: Option<String>,
}

/// Transfer error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferError {
    pub row_number: Option<u64>,
    pub statement_number: Option<u64>,
    pub message: String,
    pub sql: Option<String>,
}

/// Transfer operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub success: bool,
    pub total_rows: u64,
    pub processed_rows: u64,
    pub skipped_rows: u64,
    pub error_count: u64,
    pub duration_ms: u64,
    pub output_path: Option<String>,
    pub output_size_bytes: Option<u64>,
    pub errors: Vec<TransferError>,
}

// ── Preview & Detection ─────────────────────────────────────────

/// File format detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDetectionResult {
    pub format: ImportFormat,
    pub encoding: String,
    pub estimated_rows: Option<u64>,
    pub file_size_bytes: u64,
    pub columns: Vec<String>,
    pub csv_delimiter: Option<char>,
    pub has_header: Option<bool>,
}

/// Export preview result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows_estimate: Option<u64>,
    pub formatted_preview: String, // First N rows in target format
}

// ── Default Value Functions ─────────────────────────────────────

fn default_delimiter() -> char {
    ','
}
fn default_quote_char() -> char {
    '"'
}
fn default_encoding() -> String {
    "UTF-8".to_string()
}
fn default_lf() -> String {
    "LF".to_string()
}
fn default_true() -> bool {
    true
}
fn default_iso8601() -> String {
    "ISO8601".to_string()
}
fn default_batch_size() -> u32 {
    1000
}
fn default_import_batch_size() -> u32 {
    5000
}
fn default_sheet_name() -> String {
    "Sheet1".to_string()
}

// ── DDL Types ────────────────────────────────────────────────────

/// DDL generation options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlOptions {
    #[serde(default = "default_true")]
    pub include_create_table: bool,
    #[serde(default = "default_true")]
    pub include_primary_keys: bool,
    #[serde(default = "default_true")]
    pub include_foreign_keys: bool,
    #[serde(default = "default_true")]
    pub include_indexes: bool,
    #[serde(default = "default_true")]
    pub include_constraints: bool,
    #[serde(default)]
    pub include_comments: bool,
    #[serde(default)]
    pub include_storage_options: bool,
    #[serde(default = "default_true")]
    pub include_drop_if_exists: bool,
    #[serde(default)]
    pub include_if_not_exists: bool,
    #[serde(default)]
    pub include_data: bool,
    pub target_engine: Option<String>,
}

/// DDL request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub objects: Vec<DdlObject>,
    pub options: DdlOptions,
}

/// Object selection for DDL generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlObject {
    pub name: String,
    pub object_type: DdlObjectType,
    pub schema: Option<String>,
}

/// Object type for DDL generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DdlObjectType {
    Table,
    View,
    Index,
}

/// Index information for DDL generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub table: String,
    pub schema: Option<String>,
}

// ── Migration Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationMapping {
    pub source_column: String,
    pub source_type: String,
    pub target_column: String,
    pub target_type: String,
    pub conversion: MigrationConversion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MigrationConversion {
    Direct,
    Mapped,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTablePlan {
    pub source_table: String,
    pub target_table: String,
    pub column_mappings: Vec<MigrationMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRequest {
    pub source_connection_id: String,
    pub source_database: Option<String>,
    pub source_schema: Option<String>,
    pub target_connection_id: String,
    pub target_database: Option<String>,
    pub target_schema: Option<String>,
    pub table_plans: Vec<MigrationTablePlan>,
    #[serde(default = "default_migration_batch_size")]
    pub batch_size: u32,
    #[serde(default)]
    pub on_error: MigrationErrorStrategy,
    #[serde(default = "default_true")]
    pub create_tables: bool,
    #[serde(default)]
    pub drop_tables: bool,
    #[serde(default)]
    pub migrate_indexes: bool,
    #[serde(default)]
    pub migrate_foreign_keys: bool,
    #[serde(default = "default_true")]
    pub migrate_constraints: bool,
    #[serde(default)]
    pub disable_fk_checks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MigrationErrorStrategy {
    #[default]
    SkipRow,
    SkipTable,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPreview {
    pub tables: Vec<MigrationTablePreview>,
    pub total_rows: u64,
    pub type_conversions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTablePreview {
    pub source_table: String,
    pub target_table: String,
    pub row_count: u64,
    pub column_count: u64,
    pub mappings: Vec<MigrationMapping>,
}

fn default_migration_batch_size() -> u32 {
    5000
}

// ── Transfer Redesign Foundation Types ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSelection {
    pub server_id: String,
    pub databases: Vec<String>,
    pub schemas: HashMap<String, Vec<String>>,
    pub tables: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TransferScope {
    Table,
    Database,
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TransferProfileKind {
    Backup,
    Migrate,
    Export,
    Import,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProfile {
    pub id: String,
    pub name: String,
    pub kind: TransferProfileKind,
    pub scope: TransferScope,
    pub connection_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_connection_id: Option<String>,
    pub selection: ObjectSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ExportFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub options: serde_json::Value,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TransferJobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub stage: String,
    pub current: u64,
    pub total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferJob {
    pub id: String,
    pub name: String,
    pub kind: TransferProfileKind,
    pub scope: TransferScope,
    pub connection_id: String,
    pub status: TransferJobStatus,
    pub progress: JobProgress,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
