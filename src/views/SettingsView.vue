<script setup>
import { ref } from 'vue'
import { CheckCircle2, FolderOpen, RefreshCw, Volume2 } from '@lucide/vue'

const focusDuration = ref(25)
const breakDuration = ref(5)
const soundEnabled = ref(true)
</script>

<template>
  <section class="page settings-page">
    <header class="page-header">
      <div>
        <p class="eyebrow">Configure</p>
        <h1>Settings</h1>
        <p class="page-intro">Keep defaults simple and your local history transparent.</p>
      </div>
    </header>

    <div class="settings-stack">
      <section class="settings-card card">
        <div class="settings-card-heading">
          <div class="settings-icon"><RefreshCw :size="20" /></div>
          <div><h2>Timer defaults</h2><p>Changes apply to future timers only.</p></div>
        </div>
        <div class="settings-grid">
          <label>Default focus duration <span><input v-model="focusDuration" type="number" min="1" max="180" /> minutes</span></label>
          <label>Default break duration <span><input v-model="breakDuration" type="number" min="1" max="60" /> minutes</span></label>
        </div>
      </section>

      <section class="settings-card card">
        <div class="settings-card-heading">
          <div class="settings-icon"><FolderOpen :size="20" /></div>
          <div><h2>Obsidian vault</h2><p>FocusFlow appends task and session events to your selected folder.</p></div>
        </div>
        <div class="vault-path"><span>No vault selected in this development build</span><button class="button button-secondary" type="button">Change vault</button></div>
        <div class="delivery-status"><CheckCircle2 :size="18" /><span>0 events pending</span><button class="text-button" type="button">Retry now</button></div>
      </section>

      <section class="settings-card card sound-setting">
        <div class="settings-card-heading">
          <div class="settings-icon"><Volume2 :size="20" /></div>
          <div><h2>Timer sound</h2><p>Play a simple sound when an open timer finishes.</p></div>
        </div>
        <label class="switch"><input v-model="soundEnabled" type="checkbox" /><span aria-hidden="true" /></label>
      </section>
    </div>
  </section>
</template>
