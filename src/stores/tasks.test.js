import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useTasksStore } from './tasks.js'

const STORAGE_KEY = 'focusflow.tasks.v1'
const STORAGE_ERROR = 'Unable to save tasks. Check browser storage access and try again.'

function storedTask(overrides = {}) {
  return {
    id: 'task_123',
    title: 'Finish Dev 5 wireframes',
    course: 'Dev 5',
    status: 'active',
    createdAt: '2026-09-27T20:00:00.000Z',
    updatedAt: '2026-09-27T20:00:00.000Z',
    completedAt: null,
    ...overrides,
  }
}

describe('tasks store persistence', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('does not add a task when persistence fails and returns an actionable error', () => {
    const store = useTasksStore()
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota exceeded')
    })

    const result = store.addTask({ title: 'Read chapter 4', course: 'Dev 5' })

    expect(result).toEqual({ ok: false, errors: { persistence: STORAGE_ERROR } })
    expect(store.tasks).toEqual([])
  })

  it.each([
    JSON.stringify({ id: 'not-an-array' }),
    JSON.stringify([{}]),
    JSON.stringify([storedTask({ createdAt: null })]),
    JSON.stringify([
      storedTask({ status: 'completed', completedAt: null }),
    ]),
  ])('ignores malformed stored task data before hydration', (storedValue) => {
    localStorage.setItem(STORAGE_KEY, storedValue)
    setActivePinia(createPinia())

    const store = useTasksStore()

    expect(store.tasks).toEqual([])
    expect(store.visibleTasks).toEqual([])
  })

  it.each([
    ['update', storedTask(), (store) => store.updateTask('task_123', { title: 'Revised title', course: '' })],
    ['complete', storedTask(), (store) => store.markComplete('task_123')],
    [
      'reopen',
      storedTask({ status: 'completed', completedAt: '2026-09-27T21:00:00.000Z' }),
      (store) => store.markActive('task_123'),
    ],
    ['remove', storedTask(), (store) => store.removeTask('task_123')],
  ])('does not %s a task when persistence fails', (_operation, task, mutate) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([task]))
    setActivePinia(createPinia())
    const store = useTasksStore()
    const originalTasks = JSON.parse(JSON.stringify(store.tasks))
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota exceeded')
    })

    const result = mutate(store)

    expect(result).toEqual({ ok: false, errors: { persistence: STORAGE_ERROR } })
    expect(store.tasks).toEqual(originalTasks)
  })
})
