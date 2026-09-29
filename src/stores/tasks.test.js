import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { setBackend } from '../services/backend.js'
import { useAppStore } from './app.js'
import { useTasksStore } from './tasks.js'

const active = {
  id: 'task_active', title: 'Active task', course: 'Dev 5', status: 'active',
  createdAt: '2026-09-28T08:00:00.000Z', updatedAt: '2026-09-28T08:00:00.000Z', completedAt: null,
}
const completed = {
  ...active, id: 'task_completed', title: 'Completed task', status: 'completed',
  createdAt: '2026-09-27T08:00:00.000Z', updatedAt: '2026-09-28T09:00:00.000Z', completedAt: '2026-09-28T09:00:00.000Z',
}
const snapshot = {
  tasks: [completed, active], history: [], activeTimer: null, pendingOutboxCount: 0,
  settings: { focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: 'C:/Vault' },
}

describe('tasks store SQLite-backed adapter', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    setActivePinia(createPinia())
  })

  it('exposes active and completed tasks from the authoritative app snapshot', async () => {
    setBackend({ invoke: vi.fn().mockResolvedValue(snapshot) })
    await useAppStore().initialize()
    const store = useTasksStore()

    expect(store.activeTasks.map((task) => task.id)).toEqual(['task_active'])
    store.selectedView = 'completed'
    expect(store.visibleTasks.map((task) => task.id)).toEqual(['task_completed'])
  })

  it('keeps current state unchanged when a native mutation fails', async () => {
    const invoke = vi.fn(async (command) => {
      if (command === 'get_app_snapshot') return snapshot
      throw new Error('SQLite is read-only')
    })
    setBackend({ invoke })
    const app = useAppStore()
    await app.initialize()
    const store = useTasksStore()

    const result = await store.updateTask('task_active', { title: 'Changed', course: '' })

    expect(result.ok).toBe(false)
    expect(result.errors.persistence).toContain('SQLite is read-only')
    expect(store.tasks.find((task) => task.id === 'task_active').title).toBe('Active task')
  })
})
