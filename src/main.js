import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router/index.js'
import { useAppStore } from './stores/app.js'
import './styles/main.scss'

const application = createApp(App)
const pinia = createPinia()
application.use(pinia)
const appStore = useAppStore(pinia)
await appStore.initialize()
application.use(router)
await router.isReady()
application.mount('#app')
