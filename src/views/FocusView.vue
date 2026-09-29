<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Pause, Play, RotateCcw, Square, X } from '@lucide/vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '../stores/app.js'
import { formatClock, formatDuration } from '../domain/timer.js'

const route = useRoute()
const app = useAppStore()
const plannedMinutes = ref(app.settings.focusMinutes)
const activity = ref('')
const selectedTaskId = ref(typeof route.query.task === 'string' ? route.query.task : '')
const error = ref('')
const now = ref(new Date())
const completeConfirm = ref(false)
const cancelConfirm = ref(false)
const finishing = ref(false)
let intervalId

const timer = computed(() => app.activeTimer)
const selectedTask = computed(() => app.activeTasks.find((task) => task.id === selectedTaskId.value))
const canStart = computed(() => Boolean(selectedTask.value || activity.value.trim()))
const reading = computed(() => {
  if (!timer.value) return null
  try { return app.timerReading(now.value) } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
    return { elapsedSeconds: timer.value.accumulatedSeconds, remainingSeconds: timer.value.remainingSeconds, expired: false }
  }
})
const subject = computed(() => timer.value?.taskTitleSnapshot || timer.value?.activity || selectedTask.value?.title || activity.value || 'Choose a task or enter an activity')
const displayCourse = computed(() => timer.value ? (timer.value.kind === 'focus' ? timer.value.courseSnapshot : null) : selectedTask.value?.course)
const mode = computed(() => {
  if (!timer.value) return app.breakAvailable ? 'completed' : 'idle'
  if (timer.value.kind === 'break') return timer.value.state === 'paused' ? 'break-paused' : 'break'
  return timer.value.state
})
const timerLabel = computed(() => timer.value?.kind === 'break' ? (timer.value.state === 'paused' ? 'Break paused' : 'Break') : timer.value?.state === 'paused' ? 'Paused' : 'Focus session')
const displaySeconds = computed(() => reading.value?.remainingSeconds ?? plannedMinutes.value * 60)

async function run(action) {
  error.value = ''
  const result = await action()
  if (!result.ok) error.value = result.error || result.errors?.persistence || 'The action could not be completed.'
  return result
}

async function startFocus() {
  if (!canStart.value) return
  const result = await run(() => app.startFocus({ taskId: selectedTask.value?.id ?? null, activity: selectedTask.value ? null : activity.value, plannedMinutes: plannedMinutes.value }))
  if (result.ok) app.dismissBreak()
}

async function pause() { await run(() => app.pauseTimer(new Date())) }
async function resume() { await run(() => app.resumeTimer(new Date())) }

async function confirmCompleteEarly() {
  const result = await run(() => app.completeEarly(new Date()))
  completeConfirm.value = false
}

async function confirmCancel() {
  const isBreak = timer.value?.kind === 'break'
  const result = await run(() => isBreak ? app.cancelBreak() : app.cancelFocus(new Date()))
  cancelConfirm.value = false
  if (result.ok) app.dismissBreak()
}

async function startBreak() {
  await run(() => app.startBreak())
}

async function tick() {
  now.value = new Date()
  if (!timer.value || timer.value.state !== 'running' || finishing.value) return
  let current
  try { current = app.timerReading(now.value) } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
    return
  }
  if (!current.expired) return
  finishing.value = true
  await run(() => app.finishAtZero(now.value))
  finishing.value = false
}

watch(() => app.settings.focusMinutes, (value) => {
  if (!timer.value) plannedMinutes.value = value
})

watch(timer, (current, previous) => {
  if (previous && !current) {
    completeConfirm.value = false
    cancelConfirm.value = false
  }
})

onMounted(() => {
  intervalId = window.setInterval(tick, 250)
  tick()
})
onBeforeUnmount(() => window.clearInterval(intervalId))
</script>

<template>
  <section class="page focus-page">
    <header class="page-header compact-header">
      <div>
        <p class="eyebrow">Focus</p>
        <h1>{{ timer ? 'Stay with it' : app.breakAvailable ? 'Session complete' : 'Ready to focus?' }}</h1>
        <p class="page-intro">Choose one clear activity and give it your full attention.</p>
      </div>
    </header>

    <div v-if="error || app.recoveryError" class="blocking-alert" role="alert">
      <div><strong>Timer action failed.</strong><span>{{ error || app.recoveryError }}</span></div>
    </div>

    <div class="focus-stage" :class="`focus-stage--${mode}`">
      <div class="timer-label">{{ timerLabel }}</div>
      <div class="timer-display" :aria-label="`${displaySeconds} seconds remaining`">{{ formatClock(displaySeconds) }}</div>
      <p class="planned-duration">Planned duration · {{ formatDuration(timer?.plannedSeconds ?? plannedMinutes * 60) }}</p>

      <div v-if="!timer && !app.breakAvailable" class="focus-setup">
        <div class="duration-control">
          <label for="focus-duration">Planned duration</label>
          <span><input id="focus-duration" v-model.number="plannedMinutes" type="number" min="1" max="180" /> minutes</span>
        </div>
        <label for="focus-task">Select an active task</label>
        <select id="focus-task" v-model="selectedTaskId">
          <option value="">Standalone activity</option>
          <option v-for="task in app.activeTasks" :key="task.id" :value="task.id">{{ task.title }}</option>
        </select>
        <div v-if="!selectedTaskId" class="field">
          <label for="focus-activity">Activity description</label>
          <input id="focus-activity" v-model="activity" maxlength="120" placeholder="Study JavaScript" />
        </div>
      </div>

      <div class="focus-subject">
        <span>{{ timer?.kind === 'break' ? 'Take a moment away' : 'Working on' }}</span>
        <strong>{{ timer?.kind === 'break' ? 'Break' : subject }}</strong>
        <small v-if="displayCourse">{{ displayCourse }}</small>
        <small v-if="timer && timer.kind === 'focus'">{{ formatDuration(reading?.elapsedSeconds || 0) }} focused</small>
      </div>

      <div class="focus-controls">
        <button v-if="!timer && !app.breakAvailable" class="button button-primary button-large" type="button" data-test="start-focus" :disabled="!canStart" @click="startFocus">
          <Play :size="20" fill="currentColor" /> Start focus
        </button>
        <template v-else-if="timer?.state === 'running'">
          <button class="button button-primary button-large" type="button" @click="pause"><Pause :size="20" fill="currentColor" /> Pause</button>
          <button v-if="timer.kind === 'focus'" class="button button-secondary" type="button" @click="completeConfirm = true">Complete early</button>
          <button class="button button-ghost danger-text" type="button" @click="cancelConfirm = true"><Square :size="17" /> Cancel</button>
        </template>
        <template v-else-if="timer?.state === 'paused'">
          <button class="button button-primary button-large" type="button" @click="resume"><Play :size="20" fill="currentColor" /> Resume</button>
          <button v-if="timer.kind === 'focus'" class="button button-secondary" type="button" @click="completeConfirm = true">Complete early</button>
          <button class="button button-ghost danger-text" type="button" @click="cancelConfirm = true"><Square :size="17" /> Cancel</button>
        </template>
        <template v-else-if="app.breakAvailable">
          <button class="button button-primary button-large" type="button" @click="startBreak"><RotateCcw :size="19" /> Start break</button>
          <button class="button button-secondary" type="button" @click="app.dismissBreak()">Plan another session</button>
        </template>
        <button v-else class="button button-secondary" type="button" @click="startBreak"><RotateCcw :size="19" /> Start break</button>
      </div>
    </div>

    <div v-if="completeConfirm" class="modal-backdrop" @click.self="completeConfirm = false">
      <section class="dialog dialog-compact" role="alertdialog" aria-modal="true" aria-labelledby="complete-early-title">
        <button class="dialog-close" type="button" aria-label="Close" @click="completeConfirm = false"><X :size="20" /></button>
        <p class="eyebrow">Successful session</p>
        <h2 id="complete-early-title">Complete this session early?</h2>
        <p class="dialog-copy">Your actual focused time of {{ formatDuration(reading?.elapsedSeconds || 0) }} will be recorded.</p>
        <div class="dialog-actions"><button class="button button-ghost" type="button" @click="completeConfirm = false">Keep focusing</button><button class="button button-primary" type="button" @click="confirmCompleteEarly">Complete</button></div>
      </section>
    </div>

    <div v-if="cancelConfirm" class="modal-backdrop" @click.self="cancelConfirm = false">
      <section class="dialog dialog-compact" role="alertdialog" aria-modal="true" aria-labelledby="cancel-session-title">
        <p class="eyebrow danger-text">End timer</p>
        <h2 id="cancel-session-title">Cancel this {{ timer?.kind === 'break' ? 'break' : 'focus session' }}?</h2>
        <p class="dialog-copy">{{ timer?.kind === 'focus' ? `${formatDuration(reading?.elapsedSeconds || 0)} will remain in History but will not count toward today's completed focus time.` : 'Breaks are not stored in History.' }}</p>
        <div class="dialog-actions"><button class="button button-ghost" type="button" @click="cancelConfirm = false">Continue</button><button class="button button-danger" type="button" @click="confirmCancel">Cancel timer</button></div>
      </section>
    </div>
  </section>
</template>
