/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'

  const component: InstanceType<DefineComponent>
  export default component
}

// Monaco Editor API without built-in language contributions
// Types are identical to the main monaco-editor bundle.
declare module 'monaco-editor/esm/vs/editor/editor.api' {
  export * from 'monaco-editor'
}
