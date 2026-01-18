/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'

  const component: InstanceType<DefineComponent>
  export default component
}
