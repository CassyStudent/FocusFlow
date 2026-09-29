function finiteInteger(value) {
  return Number.isInteger(value) && Number.isFinite(value)
}

export function calculateTimer(timer, now = new Date()) {
  const planned = timer?.plannedSeconds
  const accumulated = timer?.accumulatedSeconds
  const persistedRemaining = timer?.remainingSeconds
  if (!finiteInteger(planned) || planned <= 0 || !finiteInteger(accumulated) || accumulated < 0 || accumulated > planned || !finiteInteger(persistedRemaining) || persistedRemaining < 0 || persistedRemaining > planned || accumulated + persistedRemaining !== planned) {
    throw new Error('Cannot recover an invalid persisted timer.')
  }

  if (timer.state === 'paused') {
    return { elapsedSeconds: accumulated, remainingSeconds: persistedRemaining, expired: persistedRemaining === 0, calculatedEndAt: null }
  }
  if (timer.state !== 'running' || !timer.lastResumedAt) throw new Error('Cannot recover an invalid persisted timer.')

  const resumedMs = Date.parse(timer.lastResumedAt)
  const nowMs = now.getTime()
  if (!Number.isFinite(resumedMs) || !Number.isFinite(nowMs)) throw new Error('Cannot recover an invalid timer timestamp.')
  const deltaMs = nowMs - resumedMs
  if (deltaMs <= -1000) throw new Error('Cannot recover timer because the clock moved backwards.')

  const runningSeconds = Math.max(0, Math.floor(deltaMs / 1000))
  const elapsedSeconds = Math.min(planned, accumulated + runningSeconds)
  const remainingSeconds = Math.max(0, planned - elapsedSeconds)
  const calculatedEndAt = new Date(resumedMs + persistedRemaining * 1000).toISOString()
  return { elapsedSeconds, remainingSeconds, expired: remainingSeconds === 0, calculatedEndAt }
}

export function formatClock(seconds) {
  const safe = Math.max(0, Math.floor(Number(seconds) || 0))
  const minutes = Math.floor(safe / 60)
  return `${String(minutes).padStart(2, '0')}:${String(safe % 60).padStart(2, '0')}`
}

export function formatDuration(seconds) {
  const safe = Math.max(0, Math.floor(Number(seconds) || 0))
  const hours = Math.floor(safe / 3600)
  const minutes = Math.floor((safe % 3600) / 60)
  const rest = safe % 60
  return [hours && `${hours}h`, minutes && `${minutes}m`, (rest || (!hours && !minutes)) && `${rest}s`].filter(Boolean).join(' ')
}

export function localDate(date = new Date()) {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function utcOffset(date = new Date()) {
  const total = -date.getTimezoneOffset()
  const sign = total >= 0 ? '+' : '-'
  const absolute = Math.abs(total)
  return `${sign}${String(Math.floor(absolute / 60)).padStart(2, '0')}:${String(absolute % 60).padStart(2, '0')}`
}

export function eventMeta(date = new Date(), eventId = createId('evt')) {
  return { eventId, occurredAt: date.toISOString(), eventOffset: utcOffset(date), localDate: localDate(date) }
}

export function createId(prefix) {
  const uuid = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}_${Math.random().toString(16).slice(2)}`
  return `${prefix}_${uuid}`
}
