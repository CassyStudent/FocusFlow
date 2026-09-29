import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import HistoryView from './HistoryView.vue'
import { setBackend } from '../services/backend.js'
import { useAppStore } from '../stores/app.js'

const settings = { focusMinutes: 25, breakMinutes: 5, soundEnabled: true, vaultPath: 'C:/Vault' }

function session(id, localDate, endedAt, status = 'completed') {
  return {
    id,
    taskId: null,
    activity: `Activity ${id}`,
    taskTitleSnapshot: null,
    courseSnapshot: null,
    plannedSeconds: 1500,
    actualSeconds: 300,
    status,
    completionType: status === 'completed' ? 'timer_finished' : null,
    startedAt: endedAt,
    endedAt,
    eventOffset: '+00:00',
    localDate,
  }
}

describe('HistoryView', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('groups immutable sessions by stored local end date and keeps newest sessions first', async () => {
    const history = [
      session('older-day', '2026-09-28', '2026-09-28T20:00:00.000Z'),
      session('newer', '2026-09-29', '2026-09-29T11:00:00.000Z'),
      session('older', '2026-09-29', '2026-09-29T09:00:00.000Z', 'cancelled'),
    ]
    setBackend({ invoke: async () => ({ tasks: [], history, settings, activeTimer: null, pendingOutboxCount: 0 }) })
    const store = useAppStore()
    await store.initialize(new Date(2026, 8, 29, 12, 0, 0))

    const wrapper = mount(HistoryView, { global: { stubs: { RouterLink: { template: '<a><slot /></a>' } } } })
    const groups = wrapper.findAll('[data-test="history-group"]')

    expect(groups).toHaveLength(2)
    expect(groups[0].attributes('data-date')).toBe('2026-09-29')
    expect(groups[0].text().indexOf('Activity newer')).toBeLessThan(groups[0].text().indexOf('Activity older'))
    expect(groups[1].attributes('data-date')).toBe('2026-09-28')
  })
})
