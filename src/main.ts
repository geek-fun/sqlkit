import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import { createApp } from 'vue'
import App from './App.vue'
import { setDatabaseStoreLoader } from './composables/sqlCompletion/metadata'
import { lang } from './lang'
import { router } from './router'
import { useDatabaseStore } from './store/databaseStore'
import 'virtual:uno.css'
import './assets/index.css'

const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)

const app = createApp(App)
app.use(pinia)
app.use(router)
app.use(lang)
setDatabaseStoreLoader(() => useDatabaseStore)
app.mount('#app')
