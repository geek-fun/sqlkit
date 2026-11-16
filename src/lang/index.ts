import { createI18n, useI18n } from 'vue-i18n'
import { enUS } from './enUS'
import { esES } from './esES'
import { jaJP } from './jaJP'
import { koKR } from './koKR'
import { ptBR } from './ptBR'
import { zhCN } from './zhCN'
import { zhTW } from './zhTW'

type AppLocale = 'zhCN' | 'zhTW' | 'enUS' | 'ptBR' | 'jaJP' | 'koKR' | 'esES'

function getStoredLocale(): AppLocale | 'auto' {
  if (typeof localStorage === 'undefined')
    return 'enUS'
  return (localStorage.getItem('lang') as AppLocale | 'auto') || 'auto'
}

const AVAILABLE_LOCALES: AppLocale[] = ['enUS', 'zhCN', 'zhTW', 'ptBR', 'jaJP', 'koKR', 'esES']

function detectFromNavigator(): AppLocale {
  if (typeof navigator === 'undefined')
    return 'enUS'
  const tag = (navigator.language || '').toLowerCase()
  if (/^zh-(?:tw|hk|mo|hant)/.test(tag))
    return 'zhTW'
  if (tag === 'zh-cn')
    return 'zhCN'
  if (tag.startsWith('pt'))
    return 'ptBR'
  if (tag.startsWith('ja'))
    return 'jaJP'
  if (tag.startsWith('ko'))
    return 'koKR'
  if (tag.startsWith('es'))
    return 'esES'
  return 'enUS'
}

function resolveLocale(): AppLocale {
  const stored = getStoredLocale()
  // Validate stored locale against available locales; fall back to auto if invalid
  if (stored !== 'auto' && AVAILABLE_LOCALES.includes(stored))
    return stored
  return detectFromNavigator()
}

const lang = createI18n({
  globalInjection: true,
  locale: resolveLocale(),
  fallbackLocale: 'enUS',
  legacy: false,
  missingWarn: false,
  fallbackWarn: false,
  messages: {
    zhCN,
    zhTW,
    enUS,
    ptBR,
    jaJP,
    koKR,
    esES,
  },
})

const BCP47_BY_LOCALE: Record<AppLocale, string> = {
  zhCN: 'zh-CN',
  zhTW: 'zh-TW',
  enUS: 'en-US',
  ptBR: 'pt-BR',
  jaJP: 'ja-JP',
  koKR: 'ko-KR',
  esES: 'es-ES',
}

/** Effective UI locale as a BCP-47 tag — AUTO already resolved at startup. */
const currentBcp47Locale = (): string =>
  BCP47_BY_LOCALE[lang.global.locale.value as AppLocale] ?? 'en-US'

const useLang = useI18n

export { currentBcp47Locale, detectFromNavigator, lang, useLang }
