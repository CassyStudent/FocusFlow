import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { getBackend } from '../services/backend.js'
import { calculateTimer, createId, eventMeta, localDate, utcOffset } from '../domain/timer.js'
import { normalizeTaskInput, sortTasksForView, validateTaskInput } from '../domain/tasks.js'

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error)
}

function persistenceFailure(error) {
  return { ok: false, errors: { persistence: `Unable to save locally. ${errorMessage(error)}` } }
}

function replaceById(items, value) {
  const index = items.findIndex((item) => item.id === value.id)
  return index === -1 ? [value, ...items] : items.with(index, value)
}

export const useAppStore = defineStore('app', () => {
  const backend = getBackend()
  const tasks = ref([])
  const history = ref([])
  const settings = ref({ focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: null })
  const activeTimer = ref(null)
  const pendingOutboxCount = ref(0)
  const currentLocalDate = ref(localDate())
  const breakAvailable = ref(false)
  const ready = ref(false)
  const busy = ref(false)
  const startupError = ref('')
  const recoveryError = ref('')

  const activeTasks = computed(() => sortTasksForView(tasks.value, 'active'))
  const completedTasks = computed(() => sortTasksForView(tasks.value, 'completed'))
  const needsOnboarding = computed(() => ready.value && !settings.value.vaultPath)
  const todaySummary = computed(() => {
    const completed = history.value.filter((session) => session.status === 'completed' && session.localDate === currentLocalDate.value)
    return {
      sessions: completed.length,
      seconds: completed.reduce((total, session) => total + session.actualSeconds, 0),
    }
  })

  async function call(command, args = {}) {
    return backend.invoke(command, args)
  }

  async function initialize(now = new Date()) {
    syncClock(now)
    busy.value = true
    startupError.value = ''
    try {
      const snapshot = await call('get_app_snapshot')
      tasks.value = snapshot.tasks
      history.value = snapshot.history
      settings.value = snapshot.settings
      activeTimer.value = snapshot.activeTimer
      pendingOutboxCount.value = snapshot.pendingOutboxCount
      ready.value = true
      if (activeTimer.value) await recoverTimer(now)
      if (settings.value.vaultPath && pendingOutboxCount.value > 0) await retryOutbox(now)
      return { ok: true }
    } catch (error) {
      startupError.value = errorMessage(error)
      return persistenceFailure(error)
    } finally {
      busy.value = false
    }
  }

  async function refreshSnapshot() {
    const snapshot = await call('get_app_snapshot')
    tasks.value = snapshot.tasks
    history.value = snapshot.history
    settings.value = snapshot.settings
    activeTimer.value = snapshot.activeTimer
    pendingOutboxCount.value = snapshot.pendingOutboxCount
  }

  async function createTask(input) {
    const errors = validateTaskInput(input)
    if (Object.keys(errors).length) return { ok: false, errors }
    const normalized = normalizeTaskInput(input)
    const now = new Date()
    try {
      const task = await call('create_task', { input: { id: createId('task'), ...normalized, event: eventMeta(now) } })
      tasks.value = [task, ...tasks.value]
      pendingOutboxCount.value += 1
      return { ok: true, task, errors: {} }
    } catch (error) {
      return persistenceFailure(error)
    }
  }

  async function updateTask(id, input) {
    const errors = validateTaskInput(input)
    if (Object.keys(errors).length) return { ok: false, errors }
    try {
      const task = await call('update_task', { input: { id, ...normalizeTaskInput(input), event: eventMeta() } })
      tasks.value = replaceById(tasks.value, task)
      pendingOutboxCount.value += 1
      return { ok: true, task, errors: {} }
    } catch (error) {
      return persistenceFailure(error)
    }
  }

  async function transitionTask(command, id) {
    try {
      const task = await call(command, { input: { id, event: eventMeta() } })
      tasks.value = replaceById(tasks.value, task)
      pendingOutboxCount.value += 1
      return { ok: true, task, errors: {} }
    } catch (error) {
      return persistenceFailure(error)
    }
  }

  const completeTask = (id) => transitionTask('complete_task', id)
  const reopenTask = (id) => transitionTask('reopen_task', id)

  async function deleteTask(id) {
    try {
      await call('delete_task', { input: { id, event: eventMeta() } })
      tasks.value = tasks.value.filter((task) => task.id !== id)
      pendingOutboxCount.value += 1
      return { ok: true, errors: {} }
    } catch (error) {
      return persistenceFailure(error)
    }
  }

  async function startFocus({ taskId = null, activity = null, plannedMinutes }) {
    const task = taskId ? tasks.value.find((item) => item.id === taskId && item.status === 'active') : null
    const description = activity?.trim() || null
    if ((!task && !description) || (task && description)) return { ok: false, error: 'Choose one active task or enter one activity.' }
    const minutes = Number(plannedMinutes)
    if (!Number.isInteger(minutes) || minutes < 1 || minutes > 180) return { ok: false, error: 'Focus duration must be between 1 and 180 minutes.' }
    const now = new Date()
    try {
      activeTimer.value = await call('start_focus', { input: {
        id: createId('session'), taskId: task?.id ?? null, activity: task ? null : description,
        plannedSeconds: minutes * 60, startedAt: now.toISOString(), eventOffset: utcOffset(now), localDate: localDate(now),
        event: eventMeta(now),
      } })
      breakAvailable.value = false
      pendingOutboxCount.value += 1
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function startBreak() {
    const now = new Date()
    try {
      activeTimer.value = await call('start_break', { input: {
        id: createId('break'), plannedSeconds: settings.value.breakMinutes * 60,
        startedAt: now.toISOString(), eventOffset: utcOffset(now), localDate: localDate(now),
      } })
      breakAvailable.value = false
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  function timerReading(now = new Date()) {
    return activeTimer.value ? calculateTimer(activeTimer.value, now) : null
  }

  async function pauseTimer(now = new Date()) {
    try {
      const reading = timerReading(now)
      if (reading.expired) return finishAtZero(now)
      const isFocus = activeTimer.value.kind === 'focus'
      const command = isFocus ? 'pause_focus' : 'pause_break'
      activeTimer.value = await call(command, { input: { at: now.toISOString(), accumulatedSeconds: reading.elapsedSeconds, remainingSeconds: reading.remainingSeconds, event: isFocus ? eventMeta(now) : null } })
      if (isFocus) pendingOutboxCount.value += 1
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function resumeTimer(now = new Date()) {
    try {
      const reading = timerReading(now)
      const isFocus = activeTimer.value.kind === 'focus'
      const command = isFocus ? 'resume_focus' : 'resume_break'
      activeTimer.value = await call(command, { input: { at: now.toISOString(), accumulatedSeconds: reading.elapsedSeconds, remainingSeconds: reading.remainingSeconds, event: isFocus ? eventMeta(now) : null } })
      if (isFocus) pendingOutboxCount.value += 1
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function endFocus(status, completionType, now = new Date(), endedAt = null) {
    try {
      const reading = timerReading(now)
      const endDate = endedAt ? new Date(endedAt) : now
      const input = { endedAt: endDate.toISOString(), actualSeconds: reading.elapsedSeconds, event: eventMeta(endDate) }
      const command = status === 'completed' ? 'finish_focus' : 'cancel_focus'
      if (completionType) input.completionType = completionType
      const session = await call(command, { input })
      history.value = [session, ...history.value]
      activeTimer.value = null
      breakAvailable.value = status === 'completed'
      pendingOutboxCount.value += 1
      return { ok: true, session }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  const completeEarly = (now = new Date()) => endFocus('completed', 'completed_early', now)
  const cancelFocus = (now = new Date()) => endFocus('cancelled', null, now)

  async function finishAtZero(now = new Date()) {
    if (!activeTimer.value) return { ok: true }
    let reading
    try { reading = timerReading(now) } catch (error) {
      recoveryError.value = errorMessage(error)
      return { ok: false, error: recoveryError.value }
    }
    if (!reading.expired) return { ok: true, expired: false }
    if (activeTimer.value.kind === 'break') {
      try {
        await call('cancel_break')
        activeTimer.value = null
        return { ok: true, expired: true }
      } catch (error) { return { ok: false, error: errorMessage(error) } }
    }
    return endFocus('completed', 'timer_finished', now, reading.calculatedEndAt)
  }

  async function cancelBreak() {
    try {
      await call('cancel_break')
      activeTimer.value = null
      breakAvailable.value = false
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function recoverTimer(now = new Date()) {
    recoveryError.value = ''
    try {
      const persisted = await call('recover_active_timer')
      activeTimer.value = persisted
      if (!persisted) return { ok: true }
      return finishAtZero(now)
    } catch (error) {
      recoveryError.value = errorMessage(error)
      return { ok: false, error: recoveryError.value }
    }
  }

  function syncClock(now = new Date()) {
    currentLocalDate.value = localDate(now)
  }

  function dismissBreak() {
    breakAvailable.value = false
  }

  async function saveSettings(input) {
    try {
      settings.value = await call('save_settings', { input })
      return { ok: true }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function validateVault(path) {
    try { return { ok: true, validation: await call('validate_vault', { path }) } }
    catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function configureVault(path, allowPendingRedirection = false) {
    try {
      const validation = await call('configure_vault', { input: { path, allowPendingRedirection } })
      settings.value = { ...settings.value, vaultPath: validation.path }
      return { ok: true, validation }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  async function retryOutbox(now = new Date()) {
    try {
      const result = await call('retry_outbox', { deliveredAt: now.toISOString() })
      await refreshSnapshot()
      return { ok: true, result }
    } catch (error) { return { ok: false, error: errorMessage(error) } }
  }

  return {
    tasks, history, settings, activeTimer, pendingOutboxCount, currentLocalDate, breakAvailable,
    ready, busy, startupError, recoveryError,
    activeTasks, completedTasks, needsOnboarding, todaySummary,
    initialize, refreshSnapshot, createTask, updateTask, completeTask, reopenTask, deleteTask,
    startFocus, startBreak, timerReading, pauseTimer, resumeTimer, completeEarly, cancelFocus,
    finishAtZero, cancelBreak, recoverTimer, syncClock, dismissBreak,
    saveSettings, validateVault, configureVault, retryOutbox,
  }
})
