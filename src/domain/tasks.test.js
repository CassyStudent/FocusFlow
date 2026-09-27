import { describe, expect, it } from 'vitest'
import {
  completeTask,
  createTask,
  normalizeTaskInput,
  reopenTask,
  sortTasksForView,
  validateTaskInput,
} from './tasks.js'

describe('normalizeTaskInput', () => {
  it('trims task fields and converts a blank course to null', () => {
    expect(
      normalizeTaskInput({
        title: '  Finish Dev 5 report  ',
        course: '   ',
      }),
    ).toEqual({
      title: 'Finish Dev 5 report',
      course: null,
    })
  })
})

describe('validateTaskInput', () => {
  it('rejects blank titles and fields above their approved limits', () => {
    expect(validateTaskInput({ title: '   ', course: '' })).toEqual({
      title: 'Enter a task title.',
    })

    expect(validateTaskInput({ title: 'x'.repeat(121), course: 'y'.repeat(61) })).toEqual({
      title: 'Use 120 characters or fewer.',
      course: 'Use 60 characters or fewer.',
    })
  })
})

describe('createTask', () => {
  it('creates an active task with stable identity and timestamps', () => {
    expect(
      createTask(
        { title: '  Build Tasks screen ', course: ' Dev 5 ' },
        { id: 'task_123', now: '2026-09-27T21:35:00.000Z' },
      ),
    ).toEqual({
      id: 'task_123',
      title: 'Build Tasks screen',
      course: 'Dev 5',
      status: 'active',
      createdAt: '2026-09-27T21:35:00.000Z',
      updatedAt: '2026-09-27T21:35:00.000Z',
      completedAt: null,
    })
  })
})

describe('task status transitions', () => {
  it('completes and reopens a task without changing its identity', () => {
    const task = createTask(
      { title: 'Build Tasks screen', course: 'Dev 5' },
      { id: 'task_123', now: '2026-09-27T21:35:00.000Z' },
    )

    const completed = completeTask(task, '2026-09-27T22:00:00.000Z')
    expect(completed).toMatchObject({
      id: 'task_123',
      status: 'completed',
      updatedAt: '2026-09-27T22:00:00.000Z',
      completedAt: '2026-09-27T22:00:00.000Z',
    })

    expect(reopenTask(completed, '2026-09-27T22:05:00.000Z')).toMatchObject({
      id: 'task_123',
      status: 'active',
      updatedAt: '2026-09-27T22:05:00.000Z',
      completedAt: null,
    })
  })
})

describe('sortTasksForView', () => {
  it('orders active tasks by creation and completed tasks by completion, newest first', () => {
    const tasks = [
      { id: 'a', status: 'active', createdAt: '2026-09-26T09:00:00.000Z' },
      { id: 'b', status: 'completed', createdAt: '2026-09-25T09:00:00.000Z', completedAt: '2026-09-27T10:00:00.000Z' },
      { id: 'c', status: 'active', createdAt: '2026-09-27T09:00:00.000Z' },
      { id: 'd', status: 'completed', createdAt: '2026-09-24T09:00:00.000Z', completedAt: '2026-09-27T11:00:00.000Z' },
    ]

    expect(sortTasksForView(tasks, 'active').map((task) => task.id)).toEqual(['c', 'a'])
    expect(sortTasksForView(tasks, 'completed').map((task) => task.id)).toEqual(['d', 'b'])
  })
})
