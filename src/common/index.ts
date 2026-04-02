/**
 * Convert object to plain JavaScript object (serializable)
 */
export const pureObject = <T>(obj: T): T => JSON.parse(JSON.stringify(obj)) as T

export * from './sqlParser'
