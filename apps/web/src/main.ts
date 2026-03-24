import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createOctopusI18n } from '@octopus/i18n'
import '@unocss/reset/tailwind.css'
import '@octopus/design-tokens/themes.css'
import 'virtual:uno.css'
import './main.css'
import App from './App.vue'
import { controlPlaneClientKey, createDefaultControlPlaneClient } from './lib/control-plane'
import { router } from './router'

const app = createApp(App)
const pinia = createPinia()
const i18n = createOctopusI18n()
const queryClient = new QueryClient()
const controlPlaneClient = createDefaultControlPlaneClient()

app.use(pinia)
app.use(router)
app.use(i18n)
app.use(VueQueryPlugin, { queryClient })
app.provide(controlPlaneClientKey, controlPlaneClient)
app.mount('#app')
