import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import i18n from './plugins/i18n'
import { router } from './router'
import { installStartupDiagnostics, renderStartupFailure } from './startup/diagnostics'
import { prepareRouterStartup } from './startup/router'
import '@octopus/ui/main.css'

installStartupDiagnostics()

async function bootstrapApp(): Promise<void> {
  const app = createApp(App)
  app.config.errorHandler = (error) => {
    renderStartupFailure(error)
  }

  app.use(createPinia())
  app.use(i18n)

  await prepareRouterStartup(router)

  app.use(router)
  app.mount('#app')
}

void bootstrapApp().catch((error) => {
  renderStartupFailure(error)
})
