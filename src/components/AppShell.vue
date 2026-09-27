<script setup>
import { History, ListTodo, Settings, Timer, TriangleAlert } from '@lucide/vue'

const navigation = [
  { label: 'Tasks', to: '/tasks', icon: ListTodo },
  { label: 'Focus', to: '/focus', icon: Timer },
  { label: 'History', to: '/history', icon: History },
  { label: 'Settings', to: '/settings', icon: Settings },
]

const pendingEvents = 0
const activeTimer = null
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
        <RouterLink v-if="activeTimer" class="timer-chip" to="/focus">
          <Timer :size="18" />
          <span>{{ activeTimer }}</span>
        </RouterLink>

        <div v-if="pendingEvents" class="pending-chip" role="status">
          <TriangleAlert :size="18" />
          <span>{{ pendingEvents }} Obsidian events pending</span>
        </div>

        <p class="offline-note">Offline by design</p>
      </div>
    </aside>

    <main class="main-content">
      <RouterView />
    </main>
  </div>
</template>
