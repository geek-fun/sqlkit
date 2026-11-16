/** @jest-environment jsdom */
import { createPinia, setActivePinia } from 'pinia'
import { AVAILABLE_LOCALES, detectFromNavigator, lang } from '@/lang'
import { enUS } from '@/lang/enUS'
import { esES } from '@/lang/esES'
import { jaJP } from '@/lang/jaJP'
import { koKR } from '@/lang/koKR'
import { ptBR } from '@/lang/ptBR'
import { zhCN } from '@/lang/zhCN'
import { zhTW } from '@/lang/zhTW'
import { LanguageType, useAppStore } from '@/store/appStore'

jest.mock('vue-i18n', () => ({
  createI18n: (options: { locale: string, messages: Record<string, unknown> }) => ({
    global: {
      locale: { value: options.locale },
      messages: options.messages,
    },
  }),
  useI18n: () => ({ t: (key: string) => key }),
}))

jest.mock('@/datasources/storeApi', () => ({
  storeApi: {
    getSecret: jest.fn(),
    setSecret: jest.fn(),
  },
}))

type Messages = Record<string, unknown>

const flatten = (messages: Messages, prefix = ''): string[] =>
  Object.entries(messages).flatMap(([key, value]) =>
    value && typeof value === 'object'
      ? flatten(value as Messages, `${prefix}${key}.`)
      : [`${prefix}${key}`],
  )

// Guards against locale drift: every language file must carry exactly the same
// key set as enUS, otherwise missing keys silently fall back to English at runtime.
describe('i18n locale parity', () => {
  const locales: Record<string, Messages> = { zhCN, zhTW, ptBR, jaJP, koKR, esES }

  it.each(Object.entries(locales))('%s has exactly the enUS key set', (_name, messages) => {
    const baseKeys = flatten(enUS as unknown as Messages).sort()
    const localeKeys = flatten(messages).sort()
    const missing = baseKeys.filter(key => !localeKeys.includes(key))
    const extra = localeKeys.filter(key => !baseKeys.includes(key))

    expect(missing).toEqual([])
    expect(extra).toEqual([])
  })
})

const setNavigatorLanguage = (language: string) => {
  Object.defineProperty(navigator, 'language', { value: language, configurable: true })
}

const restoreNavigatorLanguage = () => {
  delete (navigator as unknown as { language?: string }).language
}

describe('system language detection', () => {
  afterEach(restoreNavigatorLanguage)

  it.each([
    ['zh-TW', 'zhTW'],
    ['zh-HK', 'zhTW'],
    ['zh-hant-TW', 'zhTW'],
    ['zh-CN', 'zhCN'],
    ['pt-BR', 'ptBR'],
    ['pt', 'ptBR'],
    ['ja-JP', 'jaJP'],
    ['ja', 'jaJP'],
    ['ko-KR', 'koKR'],
    ['es-MX', 'esES'],
    ['en-US', 'enUS'],
    ['fr-FR', 'enUS'],
    ['zh', 'enUS'],
    ['zh-SG', 'enUS'],
  ])('maps navigator language %s to %s', (tag, expected) => {
    setNavigatorLanguage(tag)
    expect(detectFromNavigator()).toBe(expected)
  })
})

describe('language switcher', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })
  afterEach(restoreNavigatorLanguage)

  it('keeps the LanguageType choices in sync with the registered locales', () => {
    const selectableLocales = Object.values(LanguageType)
      .filter(value => value !== LanguageType.AUTO)
      .sort()
    expect(selectableLocales).toEqual([...AVAILABLE_LOCALES].sort())
  })

  it('writes the choice to localStorage.lang and applies the locale immediately', () => {
    const appStore = useAppStore()

    appStore.setLanguageType(LanguageType.JA_JP)

    expect(appStore.languageType).toBe(LanguageType.JA_JP)
    expect(localStorage.getItem('lang')).toBe('jaJP')
    expect(lang.global.locale.value).toBe('jaJP')
  })

  it('resolves AUTO to the detected system language and stores the auto marker', () => {
    setNavigatorLanguage('pt-BR')
    const appStore = useAppStore()

    appStore.setLanguageType(LanguageType.AUTO)

    expect(localStorage.getItem('lang')).toBe('auto')
    expect(lang.global.locale.value).toBe('ptBR')
  })
})
