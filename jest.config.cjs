module.exports = {
  transform: {
    '^.+\\.vue$': '<rootDir>/tests/transformers/vue3-jest-cjs.cjs',
    // Tests must stay CommonJS (jest.mock hoisting, jest globals); jest 30's
    // native require(esm) then loads ESM-only packages like pinia 4 directly.
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: { module: 'CommonJS' },
      },
    ],
  },
  transformIgnorePatterns: ['/node_modules/(?!radix-vue)/'],
  testEnvironment: 'node',
  testPathIgnorePatterns: ['/node_modules/', '/dist/'],
  globals: {
    // TypeScript 6 defaults esModuleInterop to true, which makes vue3-jest
    // wrap `jest.mock` factories returning `{ default: ... }` twice
    // (`__importDefault` on a module that already carries a `default` key).
    // The component scripts must transpile with plain `.default` accesses
    // like they did under TS 5.
    'vue-jest': { tsConfig: 'tsconfig.vue-jest.json' },
  },
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/src/$1',
    '^monaco-editor$': '<rootDir>/node_modules/monaco-editor/esm/vs/editor/editor.api.js',
  },
}
