<script setup>
import { computed, reactive, ref } from 'vue'
import { Check, CircleAlert, Edit3, Play, Plus, RotateCcw, Trash2, X } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { useTasksStore } from '../stores/tasks.js'

const store = useTasksStore()
const router = useRouter()
const createForm = reactive({ title: '', course: '' })
const createErrors = reactive({})
const editingTask = ref(null)
const editForm = reactive({ title: '', course: '' })
const editErrors = reactive({})
const deletingTask = ref(null)
const persistenceError = ref('')

const emptyTitle = computed(() =>
  store.selectedView === 'active' ? 'No active tasks' : 'No completed tasks yet',
)
const emptyCopy = computed(() =>
  store.selectedView === 'active'
    ? 'Create a task to plan your next study session.'
    : 'Completed tasks will appear here.',
)

function clearErrors(target) {
  for (const key of Object.keys(target)) delete target[key]
}

function handlePersistenceResult(result) {
  if (result.ok) {
    persistenceError.value = ''
    return true
  }

  if (result.errors.persistence) persistenceError.value = result.errors.persistence
  return false
}

function submitCreate() {
  clearErrors(createErrors)
  const result = store.addTask(createForm)
  if (!result.ok) {
    Object.assign(createErrors, result.errors)
    handlePersistenceResult(result)
    return
  }

  handlePersistenceResult(result)
  createForm.title = ''
  createForm.course = ''
}

function startEdit(task) {
  editingTask.value = task
  editForm.title = task.title
  editForm.course = task.course ?? ''
  clearErrors(editErrors)
}

function saveEdit() {
  const result = store.updateTask(editingTask.value.id, editForm)
  clearErrors(editErrors)
  if (!result.ok) {
    Object.assign(editErrors, result.errors)
    handlePersistenceResult(result)
    return
  }
  handlePersistenceResult(result)
  editingTask.value = null
}

function confirmDelete() {
  const result = store.removeTask(deletingTask.value.id)
  if (handlePersistenceResult(result)) deletingTask.value = null
}

function completeTask(taskId) {
  handlePersistenceResult(store.markComplete(taskId))
}

function reopenTask(taskId) {
  handlePersistenceResult(store.markActive(taskId))
}

function focusTask(task) {
  router.push({ name: 'focus', query: { task: task.id } })
}
</script>

<template>
  <section class="page tasks-page">
    <header class="page-header">
      <div>
        <p class="eyebrow">Plan</p>
        <h1>Tasks</h1>
        <p class="page-intro">Plan your coursework and start focused work.</p>
      </div>
    </header>

    <div class="segmented-control" aria-label="Task status">
      <button
        type="button"
        :class="{ active: store.selectedView === 'active' }"
        @click="store.selectedView = 'active'"
      >
        Active <span>{{ store.activeTasks.length }}</span>
      </button>
      <button
        type="button"
        :class="{ active: store.selectedView === 'completed' }"
        @click="store.selectedView = 'completed'"
      >
        Completed <span>{{ store.completedTasks.length }}</span>
      </button>
    </div>

    <div v-if="persistenceError" class="blocking-alert" role="alert">
      <CircleAlert :size="20" />
      <div>
        <strong>Your change was not saved.</strong>
        <span>{{ persistenceError }}</span>
      </div>
    </div>

    <form v-if="store.selectedView === 'active'" class="quick-create card" @submit.prevent="submitCreate">
      <div class="field field-title">
        <label for="task-title">Task title</label>
        <input
          id="task-title"
          v-model="createForm.title"
          name="title"
          maxlength="120"
          placeholder="What do you need to finish?"
          :aria-invalid="Boolean(createErrors.title)"
        />
        <span v-if="createErrors.title" class="field-error">{{ createErrors.title }}</span>
      </div>
      <div class="field field-course">
        <label for="task-course">Course/Subject <span>Optional</span></label>
        <input
          id="task-course"
          v-model="createForm.course"
          name="course"
          maxlength="60"
          placeholder="Dev 5"
          :aria-invalid="Boolean(createErrors.course)"
        />
        <span v-if="createErrors.course" class="field-error">{{ createErrors.course }}</span>
      </div>
      <button class="button button-primary add-task-button" type="submit">
        <Plus :size="19" /> Add task
      </button>
    </form>

    <div v-if="store.visibleTasks.length" class="task-list" aria-live="polite">
      <article v-for="task in store.visibleTasks" :key="task.id" class="task-row card">
        <div class="task-copy">
          <h2>{{ task.title }}</h2>
          <span v-if="task.course" class="course-badge">{{ task.course }}</span>
          <span v-else class="task-meta">No course</span>
        </div>

        <div v-if="task.status === 'active'" class="task-actions">
          <button class="button button-primary button-small" type="button" @click="focusTask(task)">
            <Play :size="16" fill="currentColor" /> Focus
          </button>
          <button class="icon-button" type="button" :aria-label="`Edit ${task.title}`" @click="startEdit(task)">
            <Edit3 :size="18" />
          </button>
          <button class="icon-button" type="button" :aria-label="`Complete ${task.title}`" @click="completeTask(task.id)">
            <Check :size="19" />
          </button>
          <button class="icon-button danger" type="button" :aria-label="`Delete ${task.title}`" @click="deletingTask = task">
            <Trash2 :size="18" />
          </button>
        </div>

        <div v-else class="task-actions">
          <button class="button button-secondary button-small" type="button" @click="reopenTask(task.id)">
            <RotateCcw :size="16" /> Reopen
          </button>
          <button class="icon-button danger" type="button" :aria-label="`Delete ${task.title}`" @click="deletingTask = task">
            <Trash2 :size="18" />
          </button>
        </div>
      </article>
    </div>

    <div v-else class="empty-state card">
      <div class="empty-icon"><Check :size="24" /></div>
      <h2>{{ emptyTitle }}</h2>
      <p>{{ emptyCopy }}</p>
      <button
        v-if="store.selectedView === 'completed'"
        class="button button-secondary"
        type="button"
        @click="store.selectedView = 'active'"
      >
        View active tasks
      </button>
    </div>

    <div v-if="editingTask" class="modal-backdrop" @click.self="editingTask = null">
      <section class="dialog" role="dialog" aria-modal="true" aria-labelledby="edit-task-title">
        <button class="dialog-close" type="button" aria-label="Close edit dialog" @click="editingTask = null">
          <X :size="20" />
        </button>
        <p class="eyebrow">Active task</p>
        <h2 id="edit-task-title">Edit task</h2>
        <div class="dialog-fields">
          <div class="field">
            <label for="edit-title">Task title</label>
            <input id="edit-title" v-model="editForm.title" maxlength="120" />
            <span v-if="editErrors.title" class="field-error">{{ editErrors.title }}</span>
          </div>
          <div class="field">
            <label for="edit-course">Course/Subject <span>Optional</span></label>
            <input id="edit-course" v-model="editForm.course" maxlength="60" />
            <span v-if="editErrors.course" class="field-error">{{ editErrors.course }}</span>
          </div>
        </div>
        <div class="dialog-actions">
          <button class="button button-ghost" type="button" @click="editingTask = null">Cancel</button>
          <button class="button button-primary" type="button" @click="saveEdit">Save changes</button>
        </div>
      </section>
    </div>

    <div v-if="deletingTask" class="modal-backdrop" @click.self="deletingTask = null">
      <section class="dialog dialog-compact" role="alertdialog" aria-modal="true" aria-labelledby="delete-task-title">
        <p class="eyebrow danger-text">Permanent action</p>
        <h2 id="delete-task-title">Delete this task?</h2>
        <p class="dialog-copy">
          This removes “{{ deletingTask.title }}” from FocusFlow. Existing focus history and Obsidian records will remain.
        </p>
        <div class="dialog-actions">
          <button class="button button-ghost" type="button" @click="deletingTask = null">Cancel</button>
          <button class="button button-danger" type="button" @click="confirmDelete">Delete task</button>
        </div>
      </section>
    </div>
  </section>
</template>
