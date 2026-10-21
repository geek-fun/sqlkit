import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api'

const colorToHex = (value: string): string => {
  const [hue, saturation, lightness] = value.match(/[\d.]+/g)!.map(Number)
  const light = lightness / 100
  const amplitude = saturation / 100 * Math.min(light, 1 - light)
  const channel = (offset: number) => {
    const position = (offset + hue / 30) % 12
    const value = light - amplitude * Math.max(-1, Math.min(position - 3, 9 - position, 1))
    return Math.round(value * 255).toString(16).padStart(2, '0')
  }
  return `#${channel(0)}${channel(8)}${channel(4)}`
}

export const applyEditorTheme = (monaco: typeof Monaco, dark: boolean): string => {
  const styles = getComputedStyle(document.documentElement)
  const color = (token: string) => colorToHex(styles.getPropertyValue(`--${token}`))
  const name = dark ? 'sqlkit-dark' : 'sqlkit-light'
  monaco.editor.defineTheme(name, {
    base: dark ? 'vs-dark' : 'vs',
    inherit: true,
    rules: [
      { token: 'keyword', foreground: color('syntax-keyword').slice(1) },
      { token: 'string', foreground: color('syntax-string').slice(1) },
      { token: 'number', foreground: color('syntax-number').slice(1) },
      { token: 'comment', foreground: color('syntax-comment').slice(1) },
    ].flatMap(rule => [rule, { ...rule, token: `${rule.token}.sql` }]),
    colors: {
      'editor.background': color('background'),
      'editor.foreground': color('foreground'),
      'editorGutter.background': color('background'),
      'editorLineNumber.foreground': color('muted-foreground'),
      'editorLineNumber.activeForeground': color('foreground'),
      'editor.lineHighlightBackground': color('chrome'),
      'editor.lineHighlightBorder': color('chrome'),
      'editor.selectionBackground': color('selection'),
      'editor.inactiveSelectionBackground': color('accent'),
      'editorCursor.foreground': color('primary-text'),
      'editorIndentGuide.background1': color('border'),
      'editorIndentGuide.activeBackground1': color('input'),
      'editorWidget.background': color('popover'),
      'editorWidget.border': color('border'),
      'editorSuggestWidget.background': color('popover'),
      'editorSuggestWidget.foreground': color('popover-foreground'),
      'editorSuggestWidget.border': color('border'),
      'editorSuggestWidget.selectedBackground': color('selection'),
      'editorHoverWidget.background': color('popover'),
      'editorHoverWidget.foreground': color('popover-foreground'),
      'editorHoverWidget.border': color('border'),
      'editorError.foreground': color('destructive-text'),
      'editorWarning.foreground': color('warning'),
      'focusBorder': color('ring'),
    },
  })
  monaco.editor.setTheme(name)
  return name
}
