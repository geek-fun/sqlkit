/**
 * Convert object to plain JavaScript object (serializable)
 */
export const pureObject = <T>(obj: T): T => JSON.parse(JSON.stringify(obj)) as T

export * from './agentPrompt'
export * from './aiPermissions'
export * from './avatarExpiry'
export * from './avatarRefresh'
export * from './entitlement'
export * from './sessionExpiry'
export * from './sessionLifecycle'

export * from './sessionPhase'

export * from './sqlParser'
