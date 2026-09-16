import { openUrl } from '@tauri-apps/plugin-opener'

const GEEKFUN_BASE_URL = 'https://console-geekfun.wentsen.com'
const GEEKFUN_LOCAL_URL = 'http://localhost:5174'

function getGeekfunUrl(): string {
  const isDev = import.meta.env.DEV
  return isDev ? GEEKFUN_LOCAL_URL : GEEKFUN_BASE_URL
}

export async function openLoginUrl(): Promise<void> {
  const loginUrl = `${getGeekfunUrl()}/login?source=sqlkit`
  await openUrl(loginUrl)
}

export async function openRegisterUrl(): Promise<void> {
  const registerUrl = `${getGeekfunUrl()}/register?source=sqlkit`
  await openUrl(registerUrl)
}
