import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it } from 'vitest'
import FocusView from './FocusView.vue'

describe('FocusView', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
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
})
