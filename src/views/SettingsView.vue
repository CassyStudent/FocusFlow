<script setup>
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { CheckCircle2, CircleAlert, FolderOpen, RefreshCw, Volume2 } from '@lucide/vue'
import { useAppStore } from '../stores/app.js'

const app = useAppStore()
const focusDuration = ref(app.settings.focusMinutes)
const breakDuration = ref(app.settings.breakMinutes)
const soundEnabled = ref(app.settings.soundEnabled)
const error = ref('')
const message = ref('')
const obsidianWarning = ref(false)
const saving = ref(false)

async function saveTimerSettings() {
  error.value = ''
  message.value = ''
  const focusMinutes = Number(focusDuration.value)
  const breakMinutes = Number(breakDuration.value)
  if (!Number.isInteger(focusMinutes) || focusMinutes < 1 || focusMinutes > 180 || !Number.isInteger(breakMinutes) || breakMinutes < 1 || breakMinutes > 60) {
    error.value = 'Focus must be 1–180 minutes and break must be 1–60 minutes.'
    return
  }
  saving.value = true
  const result = await app.saveSettings({ focusMinutes, breakMinutes, soundEnabled: Boolean(soundEnabled.value) })
  saving.value = false
  if (result.ok) message.value = 'Settings saved.'
  else error.value = result.error
}

async function chooseVault() {
  error.value = ''
  message.value = ''
  const path = await open({ directory: true, multiple: false, title: 'Select an Obsidian vault folder' })
  if (!path) return
  const validation = await app.validateVault(path)
  if (!validation.ok) { error.value = validation.error; return }
  let allowPendingRedirection = false
  if (app.settings.vaultPath && app.settings.vaultPath !== path && app.pendingOutboxCount > 0) {
    allowPendingRedirection = window.confirm(`${app.pendingOutboxCount} pending events will be written to the new vault. Continue?`)
    if (!allowPendingRedirection) return
  }
  const result = await app.configureVault(path, allowPendingRedirection)
  if (!result.ok) { error.value = result.error; return }
  obsidianWarning.value = result.validation.obsidianWarning
  message.value = 'Vault saved. FocusFlow/Tasks and FocusFlow/Sessions are ready.'
}

async function validateCurrentVault() {
  if (!app.settings.vaultPath) return
  const result = await app.validateVault(app.settings.vaultPath)
  if (!result.ok) error.value = result.error
  else {
    obsidianWarning.value = result.validation.obsidianWarning
    message.value = 'Vault folder is writable.'
  }
}

async function retry() {
  error.value = ''
  const result = await app.retryOutbox()
  if (!result.ok) error.value = result.error
  else message.value = `Delivered ${result.result.delivered} event(s); ${result.result.alreadyPresent} already present; ${result.result.failed} failed.`
}
</script>

<template>
  <section class="page settings-page">
    <header class="page-header"><div><p class="eyebrow">Configure</p><h1>Settings</h1><p class="page-intro">Keep defaults simple and your local history transparent.</p></div></header>

    <div v-if="error" class="blocking-alert" role="alert"><CircleAlert :size="20" /><div><strong>Settings action failed.</strong><span>{{ error }}</span></div></div>
    <div v-if="message" class="success-alert" role="status"><CheckCircle2 :size="20" /><span>{{ message }}</span></div>

    <div class="settings-stack">
      <section class="settings-card card">
        <div class="settings-card-heading"><div class="settings-icon"><RefreshCw :size="20" /></div><div><h2>Timer defaults</h2><p>Changes apply to future timers only.</p></div></div>
        <div class="settings-grid">
          <label>Default focus duration <span><input v-model.number="focusDuration" type="number" min="1" max="180" /> minutes</span></label>
          <label>Default break duration <span><input v-model.number="breakDuration" type="number" min="1" max="60" /> minutes</span></label>
        </div>
        <button class="button button-primary settings-save" type="button" :disabled="saving" @click="saveTimerSettings">Save timer settings</button>
      </section>

      <section class="settings-card card">
        <div class="settings-card-heading"><div class="settings-icon"><FolderOpen :size="20" /></div><div><h2>Obsidian vault</h2><p>FocusFlow appends task and session events to your selected folder.</p></div></div>
        <div class="vault-path"><span>{{ app.settings.vaultPath || 'No vault selected' }}</span><button class="button button-secondary" type="button" @click="chooseVault">{{ app.settings.vaultPath ? 'Change vault' : 'Select vault' }}</button></div>
        <p v-if="obsidianWarning" class="inline-warning">This folder has no .obsidian directory. It is still accepted as a writable demonstration folder.</p>
        <div class="delivery-status"><CheckCircle2 :size="18" /><span>{{ app.pendingOutboxCount }} event{{ app.pendingOutboxCount === 1 ? '' : 's' }} pending</span><button class="text-button" type="button" @click="retry">Retry now</button><button class="text-button" type="button" @click="validateCurrentVault">Validate vault</button></div>
      </section>

      <section class="settings-card card sound-setting">
        <div class="settings-card-heading"><div class="settings-icon"><Volume2 :size="20" /></div><div><h2>Timer sound</h2><p>Play a simple sound when an open timer finishes.</p></div></div>
        <label class="switch"><input v-model="soundEnabled" type="checkbox" @change="saveTimerSettings" /><span aria-hidden="true" /></label>
      </section>
    </div>
  </section>
</template>
