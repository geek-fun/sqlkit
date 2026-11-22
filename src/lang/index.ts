import { createI18n, useI18n } from 'vue-i18n'
import { enUS } from './enUS'
import { esES } from './esES'
import { jaJP } from './jaJP'
import { koKR } from './koKR'
import { ptBR } from './ptBR'
import { zhCN } from './zhCN'
import { zhTW } from './zhTW'

enum LanguageType {
  AUTO = 'auto',
  ZH_CN = 'zhCN',
  ZH_TW = 'zhTW',
  EN_US = 'enUS',
  PT_BR = 'ptBR',
  JA_JP = 'jaJP',
  KO_KR = 'koKR',
  ES_ES = 'esES',
}

type AppLocale = Exclude<LanguageType, LanguageType.AUTO>

const availableLocales = Object.values(LanguageType).filter(locale => locale !== LanguageType.AUTO)

const readLanguagePreference = (): LanguageType => {
  if (typeof localStorage === 'undefined')
    return LanguageType.AUTO
  const stored = localStorage.getItem('lang')
  return availableLocales.find(locale => locale === stored) ?? LanguageType.AUTO
}

const detectFromNavigator = (): AppLocale => {
  if (typeof navigator === 'undefined')
    return LanguageType.EN_US
  const tag = (navigator.language || '').toLowerCase()
  if (/^zh-(?:tw|hk|mo|hant)(?:-|$)/.test(tag))
    return LanguageType.ZH_TW
  if (tag === 'zh' || tag === 'zh-cn' || tag === 'zh-sg')
    return LanguageType.ZH_CN
  if (tag.startsWith('pt'))
    return LanguageType.PT_BR
  if (tag.startsWith('ja'))
    return LanguageType.JA_JP
  if (tag.startsWith('ko'))
    return LanguageType.KO_KR
  if (tag.startsWith('es'))
    return LanguageType.ES_ES
  return LanguageType.EN_US
}

const resolveLocale = (preference: LanguageType): AppLocale =>
  preference === LanguageType.AUTO
    ? detectFromNavigator()
    : preference

const lang = createI18n({
  globalInjection: true,
  locale: resolveLocale(readLanguagePreference()),
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

export { currentBcp47Locale, detectFromNavigator, lang, LanguageType, readLanguagePreference, resolveLocale, useLang }
