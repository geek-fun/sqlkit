import { createI18n, useI18n } from 'vue-i18n'
import { enUS } from './enUS'
import { zhCN } from './zhCN'

function detectLocale(): string {
  try {
    const stored = localStorage.getItem('lang') || 'auto'
    return stored === 'auto' ? (navigator.language === 'zh-CN' ? 'zhCN' : 'enUS') : stored
  } catch {
    return 'enUS'
  }
}

const lang = createI18n({
  globalInjection: true,
  locale: detectLocale(),
  legacy: false,
  messages: {
    zhCN,
    enUS,
  },
})

// Re-register locale messages on Vite HMR to prevent stale references.
// eval() prevents ts-jest from parsing import.meta as syntax in CJS.
try {
  const viteMeta: { hot?: { accept: Function } } = eval('import.meta')
  if (viteMeta.hot) {
    viteMeta.hot.accept(['./enUS', './zhCN'], ([newEnUS, newZhCN]: any) => {
      if (newEnUS)
        lang.global.setLocaleMessage('enUS', newEnUS.enUS)
      if (newZhCN)
        lang.global.setLocaleMessage('zhCN', newZhCN.zhCN)
    })
  }
}
catch {} /* eslint-disable-line no-empty */

const useLang = useI18n

export { lang, useLang }
