import { createRouter, createWebHashHistory } from 'vue-router'
import TasksView from '../views/TasksView.vue'
import FocusView from '../views/FocusView.vue'
import HistoryView from '../views/HistoryView.vue'
import SettingsView from '../views/SettingsView.vue'
import OnboardingView from '../views/OnboardingView.vue'
import { useAppStore } from '../stores/app.js'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/tasks' },
    { path: '/onboarding', name: 'onboarding', component: OnboardingView },
    { path: '/tasks', name: 'tasks', component: TasksView },
    { path: '/focus', name: 'focus', component: FocusView },
    { path: '/history', name: 'history', component: HistoryView },
    { path: '/settings', name: 'settings', component: SettingsView },
  ],
})

router.beforeEach((to) => {
  const app = useAppStore()
  if (!app.ready) return true
  if (app.needsOnboarding && to.name !== 'onboarding') return { name: 'onboarding' }
  if (!app.needsOnboarding && to.name === 'onboarding') return { name: 'tasks' }
  return true
})
