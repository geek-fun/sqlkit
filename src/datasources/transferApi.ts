import type {
  ExportPreview,
  ExportRequest,
  FileDetectionResult,
  ImportFormat,
  ImportRequest,
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
