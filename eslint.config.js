import antfu from '@antfu/eslint-config'

export default antfu({
  typescript: true,
  formatters: {
    prettierOptions: {
      singleQuote: true,
      printWidth: 100,
      tabWidth: 2,
      useTabs: false,
      semi: true,
      bracketSpacing: true,
      arrowParens: 'avoid',
      endOfLine: 'auto',
      htmlWhitespaceSensitivity: 'ignore',
      vueIndentScriptAndStyle: false,
    },
  },
  unocss: true,
  vue: true,
  ignores: [
    '**/target/**',
    '**/dist/**',
    '**/node_modules/**',
    '**/.tauri/**',
    'AGENTS.md',
  ],
  rules: {
    'no-console': 'warn',
    'unused-imports/no-unused-vars': 'warn',
  },
})
