<script setup>
import { computed, ref } from 'vue'
import { CheckCircle2, Clock3, XCircle } from '@lucide/vue'
import { useAppStore } from '../stores/app.js'
import { formatDuration } from '../domain/timer.js'

const app = useAppStore()
const filter = ref('all')
const visibleSessions = computed(() => app.history
  .filter((session) => filter.value === 'all' || session.status === filter.value)
  .toSorted((a, b) => b.endedAt.localeCompare(a.endedAt)))
const visibleGroups = computed(() => {
  const groups = new Map()
  for (const session of visibleSessions.value) {
    const items = groups.get(session.localDate) || []
    items.push(session)
    groups.set(session.localDate, items)
  }
  return [...groups].map(([date, sessions]) => ({ date, sessions }))
})

function subject(session) {
  return session.taskTitleSnapshot || session.activity
}

function endedLabel(session) {
  return new Date(session.endedAt).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
}

function dateHeading(date) {
  if (date === app.currentLocalDate) return 'Today'
  const current = new Date(`${app.currentLocalDate}T12:00:00`)
  current.setDate(current.getDate() - 1)
  const yesterday = `${current.getFullYear()}-${String(current.getMonth() + 1).padStart(2, '0')}-${String(current.getDate()).padStart(2, '0')}`
  if (date === yesterday) return 'Yesterday'
  return new Date(`${date}T12:00:00`).toLocaleDateString([], { dateStyle: 'long' })
}
</script>

<template>
  <section class="page history-page">
    <header class="page-header">
      <div>
        <p class="eyebrow">Review</p>
        <h1>History</h1>
        <p class="page-intro">Completed and cancelled focus sessions are read-only.</p>
      </div>
    </header>

    <section class="daily-summary" aria-label="Today's focus summary">
      <div>
        <p class="summary-label">Today</p>
        <strong>{{ app.todaySummary.sessions }}</strong>
        <span>completed sessions</span>
      </div>
      <div class="summary-divider" />
      <div>
        <p class="summary-label">Actual focus time</p>
        <strong>{{ formatDuration(app.todaySummary.seconds) }}</strong>
        <span>focused today</span>
      </div>
    </section>

    <div class="segmented-control history-filter" aria-label="History status">
      <button :class="{ active: filter === 'all' }" type="button" @click="filter = 'all'">All</button>
      <button :class="{ active: filter === 'completed' }" type="button" @click="filter = 'completed'">Completed</button>
      <button :class="{ active: filter === 'cancelled' }" type="button" @click="filter = 'cancelled'">Cancelled</button>
    </div>

    <div v-if="visibleSessions.length" class="history-list">
      <section v-for="group in visibleGroups" :key="group.date" class="history-group" data-test="history-group" :data-date="group.date">
        <h2 class="history-date-heading">{{ dateHeading(group.date) }}</h2>
        <article v-for="session in group.sessions" :key="session.id" class="history-row card">
          <div class="history-status" :class="`history-status--${session.status}`">
            <CheckCircle2 v-if="session.status === 'completed'" :size="20" />
            <XCircle v-else :size="20" />
          </div>
          <div class="history-copy">
            <div class="history-heading">
              <h3>{{ subject(session) }}</h3>
              <span>{{ endedLabel(session) }}</span>
            </div>
            <p v-if="session.courseSnapshot" class="course-badge">{{ session.courseSnapshot }}</p>
            <dl class="session-details">
              <div><dt>Status</dt><dd>{{ session.status === 'completed' ? (session.completionType === 'completed_early' ? 'Completed early' : 'Timer finished') : 'Cancelled' }}</dd></div>
              <div><dt>Actual</dt><dd>{{ formatDuration(session.actualSeconds) }}</dd></div>
              <div><dt>Planned</dt><dd>{{ formatDuration(session.plannedSeconds) }}</dd></div>
            </dl>
          </div>
        </article>
      </section>
    </div>

    <div v-else class="empty-state card history-empty">
      <div class="empty-icon"><Clock3 :size="24" /></div>
      <h2>No {{ filter === 'all' ? '' : filter }} focus sessions yet</h2>
      <p>Complete or cancel a focus session to build your history.</p>
      <RouterLink class="button button-primary" to="/focus"><CheckCircle2 :size="18" /> Start focusing</RouterLink>
    </div>
  </section>
</template>
