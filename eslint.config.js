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
    '.impeccable/hook.cache.json',
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
}).then(configs => [
  ...configs,
  {
    // @unocss/eslint-plugin infers the config location from each linted
    // file's path; on Windows the markdown processor's virtual code-block
    // paths use backslashes, so the inference lands on the .md file itself
    // and the plugin crashes trying to import it as a config module. Pin
    // the config path so the inference is bypassed on every platform.
    name: 'sqlkit/unocss-config-path',
    settings: {
      unocss: { configPath: './uno.config.ts' },
    },
  },
])
