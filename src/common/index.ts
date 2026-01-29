/**
 * Convert a potentially reactive object to a plain JavaScript object.
 * Useful for ensuring that data stored in the Tauri store is serializable.
 *
 * @param obj - The object to convert
 * @returns A plain JavaScript object copy
 */
export function pureObject<T>(obj: T): T {
  return JSON.parse(JSON.stringify(obj)) as T
}

export * from './sqlParser'
