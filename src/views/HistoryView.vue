<script setup>
import { CheckCircle2, Clock3 } from '@lucide/vue'
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
        <strong>0</strong>
        <span>completed sessions</span>
      </div>
      <div class="summary-divider" />
      <div>
        <p class="summary-label">Actual focus time</p>
        <strong>0m</strong>
        <span>focused today</span>
      </div>
    </section>

    <div class="segmented-control history-filter" aria-label="History status">
      <button class="active" type="button">All</button>
      <button type="button">Completed</button>
      <button type="button">Cancelled</button>
    </div>

    <div class="empty-state card history-empty">
      <div class="empty-icon"><Clock3 :size="24" /></div>
      <h2>No focus sessions yet</h2>
      <p>Complete or cancel a focus session to build your history.</p>
      <RouterLink class="button button-primary" to="/focus">
        <CheckCircle2 :size="18" /> Start focusing
      </RouterLink>
    </div>
  </section>
</template>
