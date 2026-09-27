import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TasksView from './TasksView.vue'

describe('TasksView', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('creates a title-first task and shows it in the active list', async () => {
    const wrapper = mount(TasksView, {
      global: {
        stubs: {
          RouterLink: { template: '<a><slot /></a>' },
        },
      },
    })

    await wrapper.get('[name="title"]').setValue('Finish Dev 5 wireframes')
    await wrapper.get('[name="course"]').setValue('Dev 5')
    await wrapper.get('form').trigger('submit')

    expect(wrapper.text()).toContain('Finish Dev 5 wireframes')
    expect(wrapper.text()).toContain('Dev 5')
  })

  it('keeps the form and shows a blocking message when task persistence fails', async () => {
    const wrapper = mount(TasksView, {
      global: {
        stubs: {
          RouterLink: { template: '<a><slot /></a>' },
        },
      },
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota exceeded')
    })

    await wrapper.get('[name="title"]').setValue('Finish Dev 5 wireframes')
    await wrapper.get('form').trigger('submit')

    expect(wrapper.get('[role="alert"]').text()).toContain('Unable to save tasks')
    expect(wrapper.get('[name="title"]').element.value).toBe('Finish Dev 5 wireframes')
    expect(wrapper.text()).toContain('No active tasks')
  })
})
