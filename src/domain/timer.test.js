import { describe, expect, it } from 'vitest'
import {
  calculateTimer,
  formatClock,
  formatDuration,
  localDate,
  utcOffset,
} from './timer.js'

const running = {
  state: 'running',
  plannedSeconds: 1500,
  accumulatedSeconds: 300,
  remainingSeconds: 1200,
  lastResumedAt: '2026-09-28T10:00:00.000Z',
}

describe('timer domain', () => {
  it('derives running elapsed and remaining from the last resumed UTC timestamp', () => {
    expect(calculateTimer(running, new Date('2026-09-28T10:02:30.000Z'))).toEqual({
      elapsedSeconds: 450,
      remainingSeconds: 1050,
      expired: false,
      calculatedEndAt: '2026-09-28T10:20:00.000Z',
    })
  })

  it('keeps paused timer values frozen', () => {
    expect(calculateTimer({ ...running, state: 'paused', lastResumedAt: null }, new Date('2026-09-29T10:00:00Z'))).toEqual({
      elapsedSeconds: 300,
      remainingSeconds: 1200,
      expired: false,
      calculatedEndAt: null,
    })
  })

  it('clamps an expired timer at zero with its calculated end timestamp', () => {
    expect(calculateTimer(running, new Date('2026-09-28T10:30:00.000Z'))).toEqual({
      elapsedSeconds: 1500,
      remainingSeconds: 0,
      expired: true,
      calculatedEndAt: '2026-09-28T10:20:00.000Z',
    })
  })

  it('rejects negative clock changes and impossible persisted durations', () => {
    expect(() => calculateTimer(running, new Date('2026-09-28T09:59:59Z'))).toThrow('clock moved backwards')
    expect(() => calculateTimer({ ...running, accumulatedSeconds: 1600 }, new Date('2026-09-28T10:00:00Z'))).toThrow('invalid persisted timer')
  })

  it('treats a sub-second stale UI reading as zero elapsed instead of a clock change', () => {
    expect(calculateTimer(running, new Date('2026-09-28T09:59:59.900Z'))).toEqual({
      elapsedSeconds: 300,
      remainingSeconds: 1200,
      expired: false,
      calculatedEndAt: '2026-09-28T10:20:00.000Z',
    })
  })

  it('formats clock, duration, local dates, and UTC offsets', () => {
    expect(formatClock(65)).toBe('01:05')
    expect(formatDuration(3661)).toBe('1h 1m 1s')
    const date = new Date(2026, 8, 28, 12, 0, 0)
    expect(localDate(date)).toBe('2026-09-28')
    expect(utcOffset(date)).toMatch(/^[+-]\d{2}:\d{2}$/)
  })
})
