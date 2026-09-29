import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { setBackend } from '../services/backend.js'
import TasksView from './TasksView.vue'

const snapshot = {
  tasks: [], history: [], activeTimer: null, pendingOutboxCount: 0,
  settings: { focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: 'C:/Vault' },
}

describe('TasksView', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    setActivePinia(createPinia())
  })

  it('creates a title-first task through the native repository and shows it', async () => {
    setBackend({ invoke: vi.fn(async (command, args) => {
      if (command === 'create_task') return {
        id: args.input.id, title: args.input.title, course: args.input.course, status: 'active',
        createdAt: args.input.event.occurredAt, updatedAt: args.input.event.occurredAt, completedAt: null,
      }
      return snapshot
    }) })
    const wrapper = mount(TasksView)

    await wrapper.get('[name="title"]').setValue('Finish Dev 5 wireframes')
    await wrapper.get('[name="course"]').setValue('Dev 5')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(wrapper.text()).toContain('Finish Dev 5 wireframes')
    expect(wrapper.text()).toContain('Dev 5')
  })

  it('keeps the form and shows a blocking message when SQLite persistence fails', async () => {
    setBackend({ invoke: vi.fn().mockRejectedValue(new Error('SQLite is read-only')) })
    const wrapper = mount(TasksView)

    await wrapper.get('[name="title"]').setValue('Finish Dev 5 wireframes')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(wrapper.get('[role="alert"]').text()).toContain('SQLite is read-only')
    expect(wrapper.get('[name="title"]').element.value).toBe('Finish Dev 5 wireframes')
    expect(wrapper.text()).toContain('No active tasks')
  })
})
