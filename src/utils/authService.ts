import { openUrl } from '@tauri-apps/plugin-opener'

const GEEKFUN_BASE_URL = 'https://console.geekfun.club'

export async function openLoginUrl(): Promise<void> {
  const loginUrl = `${GEEKFUN_BASE_URL}/login?source=sqlkit`
  await openUrl(loginUrl)
}

export async function openRegisterUrl(): Promise<void> {
  const registerUrl = `${GEEKFUN_BASE_URL}/register?source=sqlkit`
  await openUrl(registerUrl)
}
