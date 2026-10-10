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
    'docs/**',
    'src-tauri/gen/**',
  ],
  rules: {
    // AGENTS.md prefers const-arrow declarations; the antfu preset's
    // top-level-function rule enforces the opposite
    'antfu/top-level-function': 'off',
    'no-console': 'warn',
    'unused-imports/no-unused-vars': 'warn',
    'style/eol-last': ['error', 'always'],
    'ts/consistent-type-definitions': ['error', 'type'],
  },
})
