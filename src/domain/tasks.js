export function normalizeTaskInput({ title = '', course = '' }) {
  const normalizedTitle = String(title ?? '').trim()
  const normalizedCourse = String(course ?? '').trim()

  return {
    title: normalizedTitle,
    course: normalizedCourse || null,
  }
}

export function validateTaskInput(input) {
  const normalized = normalizeTaskInput(input)
  const errors = {}

  if (!normalized.title) {
    errors.title = 'Enter a task title.'
  } else if (normalized.title.length > 120) {
    errors.title = 'Use 120 characters or fewer.'
  }

  if (normalized.course && normalized.course.length > 60) {
    errors.course = 'Use 60 characters or fewer.'
  }

  return errors
}

export function createTask(input, { id, now }) {
  const normalized = normalizeTaskInput(input)
  const errors = validateTaskInput(normalized)

  if (Object.keys(errors).length > 0) {
    throw new Error('Cannot create an invalid task.')
  }

  return {
    id,
    ...normalized,
    status: 'active',
    createdAt: now,
    updatedAt: now,
    completedAt: null,
  }
}

export function completeTask(task, now) {
  return {
    ...task,
    status: 'completed',
    updatedAt: now,
    completedAt: now,
  }
}

export function reopenTask(task, now) {
  return {
    ...task,
    status: 'active',
    updatedAt: now,
    completedAt: null,
  }
}

export function sortTasksForView(tasks, status) {
  const timestampField = status === 'completed' ? 'completedAt' : 'createdAt'

  return tasks
    .filter((task) => task.status === status)
    .toSorted((left, right) => right[timestampField].localeCompare(left[timestampField]))
}
