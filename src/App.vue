<script setup>
import { useRoute } from 'vue-router'
import AppShell from './components/AppShell.vue'
import { useAppStore } from './stores/app.js'

const route = useRoute()
const app = useAppStore()
</script>

<template>
  <main v-if="app.startupError" class="onboarding-page">
    <section class="onboarding-card card">
      <p class="eyebrow">Local storage error</p>
      <h1>FocusFlow could not open its SQLite database.</h1>
      <p>{{ app.startupError }}</p>
      <p>Your data was not changed. Close and reopen the application after checking filesystem access.</p>
    </section>
  </main>
  <RouterView v-else-if="route.name === 'onboarding'" />
  <AppShell v-else />
</template>
