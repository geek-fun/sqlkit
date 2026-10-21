/** @jest-environment jsdom */
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { applyEditorTheme } from '@/composables/editorTheme'

const css = readFileSync(join(__dirname, '../assets/index.css'), 'utf8')
const parseTokens = (block: string) => Object.fromEntries(
  [...block.matchAll(/--([\w-]+): ([^;]+);/g)].map(([, name, value]) => [name, value]),
)
const themes = {
  light: parseTokens(css.slice(css.indexOf('{') + 1, css.indexOf('}'))),
  dark: parseTokens(css.match(/\.dark \{([^}]+)\}/)![1]),
}

const luminance = (hsl: string) => {
  const [h, s, l] = hsl.match(/[\d.]+/g)!.map(Number)
  const saturation = s / 100
  const light = l / 100
  const chroma = (1 - Math.abs(2 * light - 1)) * saturation
  const x = chroma * (1 - Math.abs((h / 60) % 2 - 1))
  const m = light - chroma / 2
  const channels = [
    [chroma, x, 0],
    [x, chroma, 0],
    [0, chroma, x],
    [0, x, chroma],
    [x, 0, chroma],
    [chroma, 0, x],
  ][Math.floor(h / 60) % 6]
  return channels.map(value => value + m)
    .map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
    .reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0)
}

const contrast = (a: string, b: string) => {
  const values = [luminance(a), luminance(b)].sort((a, b) => a - b)
  return (values[1] + 0.05) / (values[0] + 0.05)
}

describe.each(['light', 'dark'] as const)('%s theme', (name) => {
  const tokens = themes[name]

  it.each(['background', 'chrome', 'card', 'popover', 'accent', 'selection'])(
    'keeps normal and supporting text readable on %s',
    (surface) => {
      ;['foreground', 'muted-foreground', 'primary-text', 'destructive-text'].forEach((text) => {
        expect(contrast(tokens[text], tokens[surface])).toBeGreaterThanOrEqual(4.5)
      })
    },
  )

  it.each(['primary', 'destructive', 'success', 'warning'])(
    'provides a readable foreground on solid %s',
    (role) => {
      expect(contrast(tokens[`${role}-foreground`], tokens[role])).toBeGreaterThanOrEqual(4.5)
    },
  )

  it.each(['success', 'warning', 'destructive'])(
    'keeps %s feedback readable on its subdued surface',
    (role) => {
      const text = role === 'destructive' ? 'destructive-text' : role
      expect(contrast(tokens[text], tokens[`${role}-muted`])).toBeGreaterThanOrEqual(4.5)
    },
  )

  it.each(['table', 'view', 'procedure', 'function', 'database'])(
    'keeps %s recognition icons readable on neutral surfaces',
    (role) => {
      ;['background', 'chrome', 'card', 'accent'].forEach((surface) => {
        expect(contrast(tokens[`icon-${role}`], tokens[surface])).toBeGreaterThanOrEqual(4.5)
      })
    },
  )

  it.each(['keyword', 'string', 'number', 'comment'])(
    'keeps SQL %s tokens readable',
    (token) => {
      expect(contrast(tokens[`syntax-${token}`], tokens.background)).toBeGreaterThanOrEqual(4.5)
    },
  )
})

describe('monaco theme integration', () => {
  const monaco = { editor: { defineTheme: jest.fn(), setTheme: jest.fn() } }
  const applyTheme = (dark: boolean) => {
    document.documentElement.removeAttribute('style')
    Object.entries(dark ? themes.dark : themes.light).forEach(([key, value]) => {
      document.documentElement.style.setProperty(`--${key}`, value)
    })
    return applyEditorTheme(monaco as unknown as Parameters<typeof applyEditorTheme>[0], dark)
  }

  afterEach(() => {
    document.documentElement.removeAttribute('style')
    jest.clearAllMocks()
  })

  it('coordinates editor, gutter and completion surfaces with the active dark tokens', () => {
    expect(applyTheme(true)).toBe('sqlkit-dark')
    expect(monaco.editor.defineTheme).toHaveBeenCalledWith('sqlkit-dark', expect.objectContaining({
      base: 'vs-dark',
      inherit: true,
      colors: expect.objectContaining({
        'editor.background': '#16181c',
        'editorGutter.background': '#16181c',
        'editor.foreground': '#e4e7ec',
        'editorSuggestWidget.background': '#23262c',
        'editorSuggestWidget.selectedBackground': '#263750',
      }),
    }))
    expect(monaco.editor.setTheme).toHaveBeenLastCalledWith('sqlkit-dark')
  })

  it('rebuilds from the current CSS values when appearance switches in either direction', () => {
    applyTheme(true)
    applyTheme(false)
    expect(monaco.editor.defineTheme).toHaveBeenLastCalledWith('sqlkit-light', expect.objectContaining({
      base: 'vs',
      colors: expect.objectContaining({ 'editor.background': '#ffffff' }),
    }))
    expect(monaco.editor.setTheme).toHaveBeenLastCalledWith('sqlkit-light')
    applyTheme(true)
    expect(monaco.editor.setTheme).toHaveBeenLastCalledWith('sqlkit-dark')
  })
})
