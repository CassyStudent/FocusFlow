import { invoke as tauriInvoke } from '@tauri-apps/api/core'

let injectedBackend = null

function clone(value) { return JSON.parse(JSON.stringify(value)) }

function browserBackend() {
  const key = 'focusflow.browser.snapshot.v2'
  const defaults = { tasks: [], history: [], activeTimer: null, pendingOutboxCount: 0, settings: { focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: null } }
  const load = () => { try { return { ...defaults, ...JSON.parse(localStorage.getItem(key) || '{}') } } catch { return clone(defaults) } }
  const save = (state) => { localStorage.setItem(key, JSON.stringify(state)); return state }
  return { async invoke(command, args = {}) {
    const state = load(); const input = args.input
    if (command === 'get_app_snapshot') return clone(state)
    if (command === 'recover_active_timer') return clone(state.activeTimer)
    if (command === 'create_task') { const task = { id: input.id, title: input.title.trim(), course: input.course?.trim() || null, status: 'active', createdAt: input.event.occurredAt, updatedAt: input.event.occurredAt, completedAt: null }; state.tasks.unshift(task); state.pendingOutboxCount++; save(state); return task }
    if (command === 'update_task') { const i = state.tasks.findIndex(t => t.id === input.id); if (i < 0 || state.tasks[i].status !== 'active') throw new Error('Reopen this task before editing it'); state.tasks[i] = { ...state.tasks[i], title: input.title.trim(), course: input.course?.trim() || null, updatedAt: input.event.occurredAt }; state.pendingOutboxCount++; save(state); return state.tasks[i] }
    if (['complete_task','reopen_task'].includes(command)) { const i = state.tasks.findIndex(t => t.id === input.id); const completed = command === 'complete_task'; state.tasks[i] = { ...state.tasks[i], status: completed ? 'completed' : 'active', completedAt: completed ? input.event.occurredAt : null, updatedAt: input.event.occurredAt }; state.pendingOutboxCount++; save(state); return state.tasks[i] }
    if (command === 'delete_task') { state.tasks = state.tasks.filter(t => t.id !== input.id); state.pendingOutboxCount++; save(state); return null }
    if (command === 'start_focus' || command === 'start_break') { state.activeTimer = { id: input.id, kind: command === 'start_focus' ? 'focus' : 'break', state: 'running', taskId: input.taskId ?? null, activity: input.activity?.trim() || null, taskTitleSnapshot: state.tasks.find(t => t.id === input.taskId)?.title ?? null, courseSnapshot: state.tasks.find(t => t.id === input.taskId)?.course ?? null, plannedSeconds: input.plannedSeconds, accumulatedSeconds: 0, remainingSeconds: input.plannedSeconds, startedAt: input.startedAt, lastResumedAt: input.startedAt, eventOffset: input.eventOffset, localDate: input.localDate }; if (command === 'start_focus') state.pendingOutboxCount++; save(state); return clone(state.activeTimer) }
    if (['pause_focus','pause_break','resume_focus','resume_break'].includes(command)) { const resume = command.startsWith('resume'); state.activeTimer = { ...state.activeTimer, state: resume ? 'running' : 'paused', accumulatedSeconds: input.accumulatedSeconds, remainingSeconds: input.remainingSeconds, lastResumedAt: resume ? input.at : null }; if (command.endsWith('focus')) state.pendingOutboxCount++; save(state); return clone(state.activeTimer) }
    if (command === 'finish_focus' || command === 'cancel_focus') { const t = state.activeTimer; const session = { id: t.id, taskId: t.taskId, activity: t.activity, taskTitleSnapshot: t.taskTitleSnapshot, courseSnapshot: t.courseSnapshot, plannedSeconds: t.plannedSeconds, actualSeconds: input.actualSeconds, status: command === 'finish_focus' ? 'completed' : 'cancelled', completionType: input.completionType ?? null, startedAt: t.startedAt, endedAt: input.endedAt, eventOffset: input.event.eventOffset, localDate: input.event.localDate }; state.history.unshift(session); state.activeTimer = null; state.pendingOutboxCount++; save(state); return session }
    if (command === 'cancel_break') { state.activeTimer = null; save(state); return null }
    if (command === 'save_settings') { state.settings = { ...state.settings, ...input }; save(state); return clone(state.settings) }
    if (command === 'validate_vault') return { path: args.path, obsidianWarning: false }
    if (command === 'configure_vault') { state.settings.vaultPath = input.path; save(state); return { path: input.path, obsidianWarning: false } }
    if (command === 'retry_outbox') { const delivered = state.pendingOutboxCount; state.pendingOutboxCount = 0; save(state); return { delivered, alreadyPresent: 0, failed: 0 } }
    throw new Error(`Unsupported browser command: ${command}`)
  } }
}

export function setBackend(backend) { injectedBackend = backend }
export function getBackend() {
  if (injectedBackend) return injectedBackend
  if (globalThis.__TAURI_INTERNALS__) return { invoke: tauriInvoke }
  injectedBackend = browserBackend()
  return injectedBackend
}
