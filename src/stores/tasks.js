import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  completeTask,
  createTask as createTaskEntity,
  normalizeTaskInput,
  reopenTask,
  sortTasksForView,
  validateTaskInput,
} from '../domain/tasks.js'

const STORAGE_KEY = 'focusflow.tasks.v1'
const STORAGE_ERROR = 'Unable to save tasks. Check browser storage access and try again.'

function isStoredTask(task) {
  if (!task || typeof task !== 'object' || Array.isArray(task)) return false

  const hasValidCompletedAt =
    task.status === 'active'
      ? task.completedAt === null
      : task.status === 'completed' && typeof task.completedAt === 'string'

  return (
    typeof task.id === 'string' &&
    typeof task.title === 'string' &&
    (task.course === null || typeof task.course === 'string') &&
    (task.status === 'active' || task.status === 'completed') &&
    typeof task.createdAt === 'string' &&
    typeof task.updatedAt === 'string' &&
    hasValidCompletedAt
  )
}

function loadTasks() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (!stored) return []

    const parsed = JSON.parse(stored)
    return Array.isArray(parsed) && parsed.every(isStoredTask) ? parsed : []
  } catch {
    return []
  }
}

function createId() {
  if (globalThis.crypto?.randomUUID) {
    return `task_${globalThis.crypto.randomUUID()}`
  }

  return `task_${Date.now()}_${Math.random().toString(16).slice(2)}`
}

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref(loadTasks())
  const selectedView = ref('active')

  const activeTasks = computed(() => sortTasksForView(tasks.value, 'active'))
  const completedTasks = computed(() => sortTasksForView(tasks.value, 'completed'))
  const visibleTasks = computed(() =>
    selectedView.value === 'active' ? activeTasks.value : completedTasks.value,
  )

  function persistTasks(nextTasks) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(nextTasks))
      tasks.value = nextTasks
      return { ok: true, errors: {} }
    } catch {
      return { ok: false, errors: { persistence: STORAGE_ERROR } }
    }
  }

  function addTask(input) {
    const errors = validateTaskInput(input)
    if (Object.keys(errors).length > 0) return { ok: false, errors }

    const now = new Date().toISOString()
    return persistTasks([...tasks.value, createTaskEntity(input, { id: createId(), now })])
  }

  function updateTask(taskId, input) {
    const errors = validateTaskInput(input)
    if (Object.keys(errors).length > 0) return { ok: false, errors }

    const index = tasks.value.findIndex((task) => task.id === taskId)
    if (index === -1 || tasks.value[index].status !== 'active') return { ok: false, errors: {} }

    const updatedTask = {
      ...tasks.value[index],
      ...normalizeTaskInput(input),
      updatedAt: new Date().toISOString(),
    }

    return persistTasks(tasks.value.with(index, updatedTask))
  }

  function markComplete(taskId) {
    const index = tasks.value.findIndex((task) => task.id === taskId)
    if (index === -1) return { ok: false, errors: {} }

    const completedTask = completeTask(tasks.value[index], new Date().toISOString())
    return persistTasks(tasks.value.with(index, completedTask))
  }

  function markActive(taskId) {
    const index = tasks.value.findIndex((task) => task.id === taskId)
    if (index === -1) return { ok: false, errors: {} }

    const activeTask = reopenTask(tasks.value[index], new Date().toISOString())
    return persistTasks(tasks.value.with(index, activeTask))
  }

  function removeTask(taskId) {
    const nextTasks = tasks.value.filter((task) => task.id !== taskId)
    if (nextTasks.length === tasks.value.length) return { ok: false, errors: {} }

    return persistTasks(nextTasks)
  }

  return {
    tasks,
    selectedView,
    activeTasks,
    completedTasks,
    visibleTasks,
    addTask,
    updateTask,
    markComplete,
    markActive,
    removeTask,
  }
})
