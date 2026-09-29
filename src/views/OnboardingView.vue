<script setup>
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { CheckCircle2, CircleAlert, FolderOpen } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app.js'

const app = useAppStore()
const router = useRouter()
const selectedPath = ref('')
const focusMinutes = ref(app.settings.focusMinutes)
const breakMinutes = ref(app.settings.breakMinutes)
const warning = ref(false)
const error = ref('')
const busy = ref(false)

async function selectFolder() {
  error.value = ''
  const path = await open({ directory: true, multiple: false, title: 'Select your Obsidian vault' })
  if (!path) return
  const result = await app.validateVault(path)
  if (!result.ok) { error.value = result.error; return }
  selectedPath.value = result.validation.path
  warning.value = result.validation.obsidianWarning
}

async function finishSetup() {
  error.value = ''
  const focus = Number(focusMinutes.value)
  const rest = Number(breakMinutes.value)
  if (!selectedPath.value) { error.value = 'Select a writable folder before continuing.'; return }
  if (!Number.isInteger(focus) || focus < 1 || focus > 180 || !Number.isInteger(rest) || rest < 1 || rest > 60) {
    error.value = 'Focus must be 1–180 minutes and break must be 1–60 minutes.'
    return
  }
  busy.value = true
  const settingsResult = await app.saveSettings({ focusMinutes: focus, breakMinutes: rest, soundEnabled: app.settings.soundEnabled })
  if (!settingsResult.ok) { error.value = settingsResult.error; busy.value = false; return }
  const vaultResult = await app.configureVault(selectedPath.value)
  busy.value = false
  if (!vaultResult.ok) { error.value = vaultResult.error; return }
  await router.replace('/tasks')
}
</script>

<template>
  <main class="onboarding-page">
    <section class="onboarding-card card">
      <div class="brand onboarding-brand"><span class="brand-mark" aria-hidden="true"><span /></span><span>FocusFlow</span></div>
      <p class="eyebrow">Offline setup</p>
      <h1>Plan tasks. Focus clearly. Keep an auditable history.</h1>
      <p>FocusFlow stores current data locally and appends task and focus events to a folder you choose. No account or cloud connection is required.</p>

      <div v-if="error" class="blocking-alert" role="alert"><CircleAlert :size="20" /><div><strong>Setup could not continue.</strong><span>{{ error }}</span></div></div>

      <section class="onboarding-section">
        <div class="settings-card-heading"><div class="settings-icon"><FolderOpen :size="20" /></div><div><h2>Select an existing writable folder</h2><p>FocusFlow creates FocusFlow/Tasks and FocusFlow/Sessions inside it.</p></div></div>
        <button class="button button-secondary" type="button" @click="selectFolder">Choose folder</button>
        <p v-if="selectedPath" class="selected-path"><CheckCircle2 :size="17" /> {{ selectedPath }}</p>
        <p v-if="warning" class="inline-warning">No .obsidian directory was found. You may continue with this writable folder.</p>
      </section>

      <section class="onboarding-section">
        <h2>Timer defaults</h2>
        <div class="settings-grid">
          <label>Focus duration <span><input v-model.number="focusMinutes" type="number" min="1" max="180" /> minutes</span></label>
          <label>Break duration <span><input v-model.number="breakMinutes" type="number" min="1" max="60" /> minutes</span></label>
        </div>
      </section>

      <button class="button button-primary button-large" type="button" :disabled="busy || !selectedPath" @click="finishSetup">Finish setup</button>
    </section>
  </main>
</template>
