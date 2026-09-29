import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { setBackend } from '../services/backend.js'
import { useAppStore } from './app.js'

const snapshot = {
  tasks: [], history: [], activeTimer: null, pendingOutboxCount: 0,
  settings: { focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: null },
}

describe('app store backend integration', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('hydrates from the backend snapshot and exposes first-run onboarding', async () => {
    setBackend({ invoke: vi.fn().mockResolvedValue(snapshot) })
    const store = useAppStore()
    await store.initialize()
    expect(store.ready).toBe(true)
    expect(store.needsOnboarding).toBe(true)
    expect(store.settings.focusMinutes).toBe(25)
  })

  it('keeps task UI unchanged and exposes backend errors when persistence fails', async () => {
    const invoke = vi.fn(async (command) => {
      if (command === 'get_app_snapshot') return snapshot
      throw new Error('Cannot edit this task while its focus timer is active')
    })
    setBackend({ invoke })
    const store = useAppStore()
    await store.initialize()
    const result = await store.createTask({ title: 'Read', course: '' })
    expect(result.ok).toBe(false)
    expect(result.errors.persistence).toContain('focus timer')
    expect(store.tasks).toEqual([])
  })

  it('recovers an expired running focus session at its calculated end time', async () => {
    const activeTimer = {
      id: 'session_1', kind: 'focus', state: 'running', taskId: null, activity: 'Study',
      taskTitleSnapshot: null, courseSnapshot: null, plannedSeconds: 60,
      accumulatedSeconds: 0, remainingSeconds: 60, startedAt: '2026-09-28T10:00:00.000Z',
      lastResumedAt: '2026-09-28T10:00:00.000Z', eventOffset: '+00:00', localDate: '2026-09-28',
    }
    const invoke = vi.fn(async (command, args) => {
      if (command === 'get_app_snapshot') return { ...snapshot, activeTimer }
      if (command === 'recover_active_timer') return activeTimer
      if (command === 'finish_focus') return {
        id: activeTimer.id, taskId: null, activity: 'Study', taskTitleSnapshot: null,
        courseSnapshot: null, plannedSeconds: 60, actualSeconds: args.input.actualSeconds,
        status: 'completed', completionType: args.input.completionType,
        startedAt: activeTimer.startedAt, endedAt: args.input.endedAt,
        eventOffset: '+00:00', localDate: '2026-09-28',
      }
      throw new Error(command)
    })
    setBackend({ invoke })
    const store = useAppStore()

    await store.initialize(new Date('2026-09-28T10:02:00.000Z'))

    const finishCall = invoke.mock.calls.find(([command]) => command === 'finish_focus')
    expect(finishCall[1].input).toMatchObject({
      actualSeconds: 60,
      completionType: 'timer_finished',
      endedAt: '2026-09-28T10:01:00.000Z',
    })
    expect(store.activeTimer).toBeNull()
    expect(store.history[0].status).toBe('completed')
  })

  it('keeps paused timers frozen during recovery', async () => {
    const activeTimer = {
      id: 'session_paused', kind: 'focus', state: 'paused', taskId: null, activity: 'Study',
      taskTitleSnapshot: null, courseSnapshot: null, plannedSeconds: 600,
      accumulatedSeconds: 120, remainingSeconds: 480, startedAt: '2026-09-28T10:00:00.000Z',
      lastResumedAt: null, eventOffset: '+00:00', localDate: '2026-09-28',
    }
    const invoke = vi.fn(async (command) => command === 'get_app_snapshot' ? { ...snapshot, activeTimer } : activeTimer)
    setBackend({ invoke })
    const store = useAppStore()

    await store.initialize(new Date('2026-09-29T10:00:00.000Z'))

    expect(store.activeTimer.state).toBe('paused')
    expect(store.timerReading(new Date('2026-10-01T10:00:00.000Z')).remainingSeconds).toBe(480)
    expect(invoke.mock.calls.some(([command]) => command === 'finish_focus')).toBe(false)
  })

  it('daily summary counts only completed sessions and sums stored seconds before formatting', async () => {
    const history = [
      { id: 'a', status: 'completed', localDate: '2026-09-28', actualSeconds: 61 },
      { id: 'b', status: 'completed', localDate: '2026-09-28', actualSeconds: 59 },
      { id: 'c', status: 'cancelled', localDate: '2026-09-28', actualSeconds: 300 },
    ]
    setBackend({ invoke: vi.fn().mockResolvedValue({ ...snapshot, history }) })
    const store = useAppStore()
    await store.initialize(new Date('2026-09-28T12:00:00.000Z'))

    expect(store.todaySummary).toEqual({ sessions: 2, seconds: 120 })
  })

  it('rolls the daily summary over when the application clock crosses midnight', async () => {
    const history = [
      { id: 'old', status: 'completed', localDate: '2026-09-29', actualSeconds: 90 },
      { id: 'new', status: 'completed', localDate: '2026-09-30', actualSeconds: 120 },
    ]
    setBackend({ invoke: vi.fn().mockResolvedValue({ ...snapshot, history }) })
    const store = useAppStore()
    await store.initialize(new Date('2026-09-29T21:59:00.000Z'))
    expect(store.todaySummary).toEqual({ sessions: 1, seconds: 90 })

    store.syncClock(new Date('2026-09-30T00:01:00.000Z'))

    expect(store.todaySummary).toEqual({ sessions: 1, seconds: 120 })
  })

  it('completes an expired focus instead of persisting a paused zero-second timer', async () => {
    const activeTimer = {
      id: 'session-expired', kind: 'focus', state: 'running', taskId: null, activity: 'Study',
      taskTitleSnapshot: null, courseSnapshot: null, plannedSeconds: 60,
      accumulatedSeconds: 0, remainingSeconds: 60, startedAt: '2026-09-29T10:00:00.000Z',
      lastResumedAt: '2026-09-29T10:00:00.000Z', eventOffset: '+00:00', localDate: '2026-09-29',
    }
    const invoke = vi.fn(async (command, args) => {
      if (command === 'get_app_snapshot') return { ...snapshot, activeTimer }
      if (command === 'recover_active_timer') return activeTimer
      if (command === 'finish_focus') return {
        id: activeTimer.id, taskId: null, activity: 'Study', taskTitleSnapshot: null,
        courseSnapshot: null, plannedSeconds: 60, actualSeconds: 60, status: 'completed',
        completionType: args.input.completionType, startedAt: activeTimer.startedAt,
        endedAt: args.input.endedAt, eventOffset: '+00:00', localDate: '2026-09-29',
      }
      if (command === 'pause_focus') throw new Error('pause_focus must not be called')
      throw new Error(command)
    })
    setBackend({ invoke })
    const store = useAppStore()
    await store.initialize(new Date('2026-09-29T10:00:30.000Z'))

    const result = await store.pauseTimer(new Date('2026-09-29T10:01:00.000Z'))

    expect(result.ok).toBe(true)
    expect(store.activeTimer).toBeNull()
    expect(store.breakAvailable).toBe(true)
    expect(invoke.mock.calls.some(([command]) => command === 'pause_focus')).toBe(false)
  })

  it('offers a break after an expired focus is completed during startup recovery', async () => {
    const activeTimer = {
      id: 'session-recovered', kind: 'focus', state: 'running', taskId: null, activity: 'Study',
      taskTitleSnapshot: null, courseSnapshot: null, plannedSeconds: 60,
      accumulatedSeconds: 0, remainingSeconds: 60, startedAt: '2026-09-29T10:00:00.000Z',
      lastResumedAt: '2026-09-29T10:00:00.000Z', eventOffset: '+00:00', localDate: '2026-09-29',
    }
    const invoke = vi.fn(async (command, args) => {
      if (command === 'get_app_snapshot') return { ...snapshot, activeTimer }
      if (command === 'recover_active_timer') return activeTimer
      if (command === 'finish_focus') return {
        id: activeTimer.id, taskId: null, activity: 'Study', taskTitleSnapshot: null,
        courseSnapshot: null, plannedSeconds: 60, actualSeconds: 60, status: 'completed',
        completionType: args.input.completionType, startedAt: activeTimer.startedAt,
        endedAt: args.input.endedAt, eventOffset: '+00:00', localDate: '2026-09-29',
      }
      throw new Error(command)
    })
    setBackend({ invoke })
    const store = useAppStore()

    await store.initialize(new Date('2026-09-29T10:02:00.000Z'))

    expect(store.breakAvailable).toBe(true)
  })
})
