import { invoke } from '@tauri-apps/api/core'

export type JreStatus = {
  installed: boolean
  version: string | null
  path: string | null
  source: 'managed' | 'system' | 'none'
}

export type DriverInfo = {
  db_type: string
  name: string
  driver_count: number
  installed: boolean
}

export const jdbcApi = {
  checkJreStatus: () => invoke<JreStatus>('check_jre_status'),

  downloadJre: () => invoke<void>('download_jre'),

  removeJre: () => invoke<void>('remove_jre'),

  listDrivers: () => invoke<DriverInfo[]>('list_drivers'),

  downloadDriver: (dbType: string) => invoke<void>('download_driver', { dbType }),

  removeDriver: (dbType: string) => invoke<void>('remove_driver', { dbType }),
}
