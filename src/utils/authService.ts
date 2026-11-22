import { openUrl } from '@tauri-apps/plugin-opener'

export const GEEKFUN_BASE_URL = 'https://console.geekfun.club'

// hosts a console account avatar may be served from — uploads land on the
// media OSS bucket, Google-registered accounts keep the Google CDN URL
const AVATAR_MEDIA_HOSTS = [
  'wentsen-media-store-prod.oss-ap-southeast-1.aliyuncs.com',
  'lh3.googleusercontent.com',
  'lh4.googleusercontent.com',
  'lh5.googleusercontent.com',
  'lh6.googleusercontent.com',
]

const ALLOWED_AVATAR_HOSTS = [
  'console.geekfun.club',
  'console-geekfun.wentsen.com',
  ...AVATAR_MEDIA_HOSTS,
]

/**
 * Render only https URLs on an exact-match allowlist — a handoff value is
 * still remote input; `allowDevHosts` enables the local dev console.
 */
export const isSafeAvatarUrl = (url: string, allowDevHosts = false): boolean => {
  try {
    const parsed = new URL(url)
    const hostAllowed = ALLOWED_AVATAR_HOSTS.includes(parsed.hostname)
      || (allowDevHosts && parsed.hostname === 'localhost')
    const protocolAllowed = parsed.protocol === 'https:'
      || (allowDevHosts && parsed.protocol === 'http:')
    return hostAllowed && protocolAllowed
  }
  catch {
    return false
  }
}

export async function openLoginUrl(): Promise<void> {
  const loginUrl = `${GEEKFUN_BASE_URL}/login?source=sqlkit`
  await openUrl(loginUrl)
}

export async function openRegisterUrl(): Promise<void> {
  const registerUrl = `${GEEKFUN_BASE_URL}/register?source=sqlkit`
  await openUrl(registerUrl)
}
