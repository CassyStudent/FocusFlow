<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { History, ListTodo, Settings, Timer, TriangleAlert } from '@lucide/vue'
import { useAppStore } from '../stores/app.js'
import { formatClock } from '../domain/timer.js'
import { useRoute } from 'vue-router'

const app = useAppStore()
const route = useRoute()
const now = ref(new Date())
let intervalId
let lastAutoRetry = 0
let retrying = false

const navigation = [
  { label: 'Tasks', to: '/tasks', icon: ListTodo },
  { label: 'Focus', to: '/focus', icon: Timer },
  { label: 'History', to: '/history', icon: History },
  { label: 'Settings', to: '/settings', icon: Settings },
]

const activeTimerLabel = computed(() => {
  if (!app.activeTimer) return ''
  try {
    const reading = app.timerReading(now.value)
    const label = app.activeTimer.kind === 'break' ? 'Break' : 'Focus'
    return `${label} · ${formatClock(reading.remainingSeconds)}`
  } catch {
    return 'Timer needs attention'
  }
})

async function updateClock() {
  now.value = new Date()
  app.syncClock(now.value)
  if (route.name !== 'focus' && app.activeTimer?.state === 'running') {
    await app.finishAtZero(now.value)
  }
  if (
    app.settings.vaultPath &&
    app.pendingOutboxCount > 0 &&
    !retrying &&
    now.value.getTime() - lastAutoRetry >= 30_000
  ) {
    retrying = true
    lastAutoRetry = now.value.getTime()
    await app.retryOutbox(now.value)
    retrying = false
  }
}

onMounted(() => { intervalId = window.setInterval(updateClock, 1000) })
onBeforeUnmount(() => window.clearInterval(intervalId))
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <RouterLink class="brand" to="/tasks" aria-label="FocusFlow tasks">
        <span class="brand-mark" aria-hidden="true"><span /></span>
        <span>FocusFlow</span>
      </RouterLink>

      <nav class="primary-nav" aria-label="Main navigation">
        <RouterLink v-for="item in navigation" :key="item.to" :to="item.to" class="nav-link">
          <component :is="item.icon" :size="20" :stroke-width="1.9" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <div class="sidebar-status">
        <RouterLink v-if="app.activeTimer" class="timer-chip" to="/focus">
          <Timer :size="18" />
          <span>{{ activeTimerLabel }}</span>
        </RouterLink>

        <div v-if="app.pendingOutboxCount" class="pending-chip" role="status">
          <TriangleAlert :size="18" />
          <span>{{ app.pendingOutboxCount }} Obsidian events pending</span>
        </div>

        <p class="offline-note">Offline by design</p>
      </div>
    </aside>

    <main class="main-content">
      <RouterView />
    </main>
  </div>
</template>
