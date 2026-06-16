import { createI18n, useI18n } from 'vue-i18n'
import { enUS } from './enUS'
import { zhCN } from './zhCN'

const langType = localStorage.getItem('lang') || 'auto'
let langName = langType
if (langType === 'auto') {
  langName = navigator.language === 'zh-CN' ? 'zhCN' : 'enUS'
}

const lang = createI18n({
  globalInjection: true,
  locale: langName,
  legacy: false,
  messages: {
    zhCN,
    enUS,
  },
})

// Re-register locale messages on Vite HMR to prevent stale references
if (import.meta.hot) {
  import.meta.hot.accept(['./enUS', './zhCN'], ([newEnUS, newZhCN]) => {
    if (newEnUS)
      lang.global.setLocaleMessage('enUS', newEnUS.enUS)
    if (newZhCN)
      lang.global.setLocaleMessage('zhCN', newZhCN.zhCN)
  })
}

const useLang = useI18n

export { lang, useLang }
