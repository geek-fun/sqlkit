import { createI18n, useI18n } from 'vue-i18n'
import { enUS } from './enUS'
import { zhCN } from './zhCN'

type AppLocale = 'zhCN' | 'enUS'

const getStoredLocale = (): AppLocale | 'auto' => {
  if (typeof localStorage === 'undefined')
    return 'enUS'
  return (localStorage.getItem('lang') as AppLocale | 'auto') || 'auto'
}

const AVAILABLE_LOCALES: AppLocale[] = ['enUS', 'zhCN']

const resolveLocale = (): AppLocale => {
  const stored = getStoredLocale()
  if (stored === 'auto')
    return typeof navigator !== 'undefined' && navigator.language === 'zh-CN' ? 'zhCN' : 'enUS'
  // Validate stored locale against available locales; fall back to auto if invalid
  if (AVAILABLE_LOCALES.includes(stored as AppLocale))
    return stored
  return typeof navigator !== 'undefined' && navigator.language === 'zh-CN' ? 'zhCN' : 'enUS'
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
    enUS,
  },
})

const useLang = useI18n

export { lang, useLang }
