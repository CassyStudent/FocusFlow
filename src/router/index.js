import { createRouter, createWebHashHistory } from 'vue-router'
import TasksView from '../views/TasksView.vue'
import FocusView from '../views/FocusView.vue'
import HistoryView from '../views/HistoryView.vue'
import SettingsView from '../views/SettingsView.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/tasks' },
    { path: '/tasks', name: 'tasks', component: TasksView },
    { path: '/focus', name: 'focus', component: FocusView },
    { path: '/history', name: 'history', component: HistoryView },
    { path: '/settings', name: 'settings', component: SettingsView },
  ],
})
