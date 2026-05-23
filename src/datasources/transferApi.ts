import type {
  DdlRequest,
  ExportFormat,
  ExportPreview,
  ExportRequest,
  FileDetectionResult,
  ImportFormat,
  ImportRequest,
  MigrationMapping,
  MigrationPreview,
  MigrationRequest,
  ObjectSelection,
  TransferProfile,
  TransferResult,
} from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'

export function previewExport(request: ExportRequest, previewRows = 10) {
  return invoke<ExportPreview>('preview_export_data', { request, previewRows })
}

export function executeExport(request: ExportRequest) {
  return invoke<TransferResult>('execute_export_data', { request })
}

export function detectFile(filePath: string) {
  return invoke<FileDetectionResult>('detect_file_format', { filePath })
}

export function previewImport(filePath: string, format: ImportFormat, previewRows = 10) {
  return invoke<ExportPreview>('preview_import_data', { filePath, format, previewRows })
}

export function executeImport(request: ImportRequest) {
  return invoke<TransferResult>('execute_import_data', { request })
}

export function previewMigration(request: MigrationRequest) {
  return invoke<MigrationPreview>('preview_migration_data', { request })
}

export function executeMigration(request: MigrationRequest) {
  return invoke<TransferResult>('execute_migration_data', { request })
}

type AutoMapParams = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  targetEngine: string
}

export function autoMapMigrationColumns(params: AutoMapParams) {
  return invoke<MigrationMapping[]>('auto_map_migration_columns', params)
}

export function generateDdl(request: DdlRequest) {
  return invoke<string>('generate_ddl_for_objects', { request })
}

export function backupServer(
  connectionId: string,
  selection: ObjectSelection,
  format: ExportFormat,
  destination: string,
  options: Record<string, unknown>,
  jobId?: string,
) {
  return invoke<string>('backup_server', {
    connectionId,
    selection,
    format,
    destination,
    options,
    jobId,
  })
}

export function migrateServer(
  sourceConnectionId: string,
  targetConnectionId: string,
  selection: ObjectSelection,
  options: Record<string, unknown>,
  jobId?: string,
) {
  return invoke<string>('migrate_server', {
    sourceConnectionId,
    targetConnectionId,
    selection,
    options,
    jobId,
  })
}

export function saveTransferProfile(profile: TransferProfile) {
  return invoke<string>('save_transfer_profile', { profile })
}

export function listTransferProfiles() {
  return invoke<TransferProfile[]>('list_transfer_profiles')
}

export function runTransferProfile(profileId: string) {
  return invoke<string>('run_transfer_profile', { profileId })
}
