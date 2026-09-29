import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useAppStore } from './app.js'

export const useTasksStore = defineStore('tasks', () => {
  const app = useAppStore()
  const selectedView = ref('active')
  const activeTasks = computed(() => app.activeTasks)
  const completedTasks = computed(() => app.completedTasks)
  const visibleTasks = computed(() => selectedView.value === 'active' ? activeTasks.value : completedTasks.value)

  return {
    tasks: computed(() => app.tasks),
    selectedView,
    activeTasks,
    completedTasks,
    visibleTasks,
    addTask: app.createTask,
    updateTask: app.updateTask,
    markComplete: app.completeTask,
    markActive: app.reopenTask,
    removeTask: app.deleteTask,
  }
})
