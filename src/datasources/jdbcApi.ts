import { invoke } from '@tauri-apps/api/core'

export type JreStatus = {
  installed: boolean
  version: string | null
  path: string | null
  source: 'managed' | 'system' | 'none'
}

export type JreUpdateStatus = {
  current_version: string | null
  latest_version: string | null
  update_available: boolean
}

export type DriverInfo = {
  db_type: string
  name: string
  installed: boolean
  version_cap: string | null
  filename: string | null
  file_size: number | null
  resolved_version: string | null
}

export type BridgeStatus = {
  installed: boolean
  current_version: string
  path: string | null
}

export const jdbcApi = {
  checkJreStatus: () => invoke<JreStatus>('check_jre_status'),

  downloadJre: () => invoke<void>('download_jre'),

  removeJre: () => invoke<void>('remove_jre'),

  checkJreUpdate: () => invoke<JreUpdateStatus>('check_jre_update'),

  checkBridgeStatus: () => invoke<BridgeStatus>('check_bridge_status'),

  downloadBridgeJar: () => invoke<void>('download_bridge_jar'),

  removeBridgeJar: () => invoke<void>('remove_bridge_jar'),

  listDrivers: () => invoke<DriverInfo[]>('list_drivers'),

  downloadDriver: (dbType: string) => invoke<void>('download_driver', { dbType }),

  removeDriver: (dbType: string) => invoke<void>('remove_driver', { dbType }),

  getJdbcNeeded: () => invoke<boolean>('get_jdbc_needed'),

  setJdbcNeeded: (needed: boolean) => invoke<void>('set_jdbc_needed', { needed }),
}
