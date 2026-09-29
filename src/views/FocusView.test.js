import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import FocusView from './FocusView.vue'
import { setBackend } from '../services/backend.js'
import { useAppStore } from '../stores/app.js'

const settings = {
  focusMinutes: 25,
  breakMinutes: 5,
  soundEnabled: true,
  vaultPath: null,
  setupComplete: true,
}

describe('FocusView', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.useRealTimers()
    setBackend(null)
  })

  it('requires a task or activity before starting a focus session', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: FocusView }],
    })
    await router.push('/')
    await router.isReady()

    const wrapper = mount(FocusView, {
      global: {
        plugins: [router],
      },
    })

    const startButton = wrapper.get('[data-test="start-focus"]')
    expect(startButton.attributes('disabled')).toBeDefined()

    await wrapper.get('#focus-activity').setValue('Study JavaScript')
    expect(startButton.attributes('disabled')).toBeUndefined()
  })

  it('uses one timestamp for a newly started timer and its first reading', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-09-28T10:00:00.000Z'))
    const task = {
      id: 'task-1',
      title: 'Write report',
      course: 'Dev 5',
      status: 'active',
      createdAt: '2026-09-28T09:00:00.000Z',
      updatedAt: '2026-09-28T09:00:00.000Z',
      completedAt: null,
    }
    setBackend({
      async invoke(command, args) {
        const input = args?.input
        if (command === 'get_app_snapshot') {
          return { tasks: [task], history: [], settings, activeTimer: null, pendingOutboxCount: 0 }
        }
        if (command === 'start_focus') {
          return {
            id: input.id,
            kind: 'focus',
            state: 'running',
            taskId: task.id,
            taskTitleSnapshot: task.title,
            courseSnapshot: task.course,
            activity: null,
            plannedSeconds: input.plannedSeconds,
            accumulatedSeconds: 0,
            remainingSeconds: input.plannedSeconds,
            startedAt: input.startedAt,
            lastResumedAt: input.startedAt,
            pausedAt: null,
          }
        }
        throw new Error(`Unexpected command: ${command}`)
      },
    })

    const store = useAppStore()
    await store.initialize(new Date())
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: FocusView }],
    })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(FocusView, { global: { plugins: [router] } })

    vi.setSystemTime(new Date('2026-09-28T10:00:00.100Z'))
    await wrapper.get('[data-test="start-focus"]').trigger('click')
    await Promise.resolve()

    expect(store.recoveryError).toBe('')
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  })

  it('dismisses confirmation dialogs when a running timer finishes naturally', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-09-28T10:00:00.000Z'))
    const timer = {
      id: 'session-1', kind: 'focus', state: 'running', taskId: null,
      taskTitleSnapshot: null, courseSnapshot: null, activity: 'Study',
      plannedSeconds: 1, accumulatedSeconds: 0, remainingSeconds: 1,
      startedAt: '2026-09-28T10:00:00.000Z', lastResumedAt: '2026-09-28T10:00:00.000Z', pausedAt: null,
    }
    setBackend({
      async invoke(command, args) {
        const input = args?.input
        if (command === 'get_app_snapshot') {
          return { tasks: [], history: [], settings, activeTimer: timer, pendingOutboxCount: 0 }
        }
        if (command === 'recover_active_timer') return timer
        if (command === 'finish_focus') {
          return {
            id: timer.id, taskId: null, activity: timer.activity,
            taskTitleSnapshot: null, courseSnapshot: null,
            plannedSeconds: 1, actualSeconds: input.actualSeconds,
            status: 'completed', completionType: input.completionType,
            startedAt: timer.startedAt, endedAt: input.endedAt,
            eventOffset: input.event.eventOffset, localDate: input.event.localDate,
          }
        }
        throw new Error(`Unexpected command: ${command}`)
      },
    })

    const store = useAppStore()
    await store.initialize(new Date())
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: FocusView }] })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(FocusView, { global: { plugins: [router] } })

    const completeEarlyButton = wrapper.findAll('button').find((button) => button.text().includes('Complete early'))
    await completeEarlyButton.trigger('click')
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(true)
    store.activeTimer = null
    await nextTick()

    expect(store.activeTimer).toBeNull()
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
  })

  it('does not show the previously selected task course during a break', async () => {
    const task = {
      id: 'task-1', title: 'Write report', course: 'Dev 5', status: 'active',
      createdAt: '2026-09-28T09:00:00.000Z', updatedAt: '2026-09-28T09:00:00.000Z', completedAt: null,
    }
    const breakTimer = {
      id: 'break-1', kind: 'break', state: 'paused', taskId: null,
      taskTitleSnapshot: null, courseSnapshot: null, activity: null,
      plannedSeconds: 300, accumulatedSeconds: 5, remainingSeconds: 295,
      startedAt: '2026-09-28T10:00:00.000Z', lastResumedAt: null, pausedAt: '2026-09-28T10:00:05.000Z',
    }
    setBackend({
      async invoke(command) {
        if (command === 'get_app_snapshot') {
          return { tasks: [task], history: [], settings, activeTimer: breakTimer, pendingOutboxCount: 0 }
        }
        if (command === 'recover_active_timer') return breakTimer
        throw new Error(`Unexpected command: ${command}`)
      },
    })
    const store = useAppStore()
    await store.initialize(new Date('2026-09-28T10:00:05.000Z'))
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: FocusView }] })
    await router.push('/?task=task-1')
    await router.isReady()
    const wrapper = mount(FocusView, { global: { plugins: [router] } })

    expect(wrapper.text()).toContain('Break')
    expect(wrapper.text()).not.toContain('Dev 5')
  })

  it('renders the persisted course snapshot from the native timer contract', async () => {
    const timer = {
      id: 'session-course', kind: 'focus', state: 'paused', taskId: 'task-1',
      taskTitleSnapshot: 'Write report', courseSnapshot: 'Dev 5', activity: null,
      plannedSeconds: 1500, accumulatedSeconds: 60, remainingSeconds: 1440,
      startedAt: '2026-09-29T10:00:00.000Z', lastResumedAt: null,
      eventOffset: '+00:00', localDate: '2026-09-29',
    }
    setBackend({
      async invoke(command) {
        if (command === 'get_app_snapshot') return { tasks: [], history: [], settings, activeTimer: timer, pendingOutboxCount: 0 }
        if (command === 'recover_active_timer') return timer
        throw new Error(`Unexpected command: ${command}`)
      },
    })
    const store = useAppStore()
    await store.initialize(new Date('2026-09-29T10:02:00.000Z'))
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: FocusView }] })
    await router.push('/')
    await router.isReady()

    const wrapper = mount(FocusView, { global: { plugins: [router] } })

    expect(wrapper.text()).toContain('Dev 5')
  })
})
