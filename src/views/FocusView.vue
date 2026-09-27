<script setup>
import { computed, ref } from 'vue'
import { Pause, Play, RotateCcw, Square } from '@lucide/vue'
import { useRoute } from 'vue-router'
import { useTasksStore } from '../stores/tasks.js'

const route = useRoute()
const tasksStore = useTasksStore()
const mode = ref('idle')
const minutes = ref(25)
const activity = ref('')
const selectedTaskId = ref(route.query.task ?? '')

const selectedTask = computed(() =>
  tasksStore.activeTasks.find((task) => task.id === selectedTaskId.value),
)
const subject = computed(() => selectedTask.value?.title || activity.value || 'Choose a task or enter an activity')
const canStart = computed(() => Boolean(selectedTask.value || activity.value.trim()))

function startFocus() {
  if (!canStart.value) return
  mode.value = 'running'
}
</script>

<template>
  <section class="page focus-page">
    <header class="page-header compact-header">
      <div>
        <p class="eyebrow">Focus</p>
        <h1>Ready to focus?</h1>
        <p class="page-intro">Choose one clear activity and give it your full attention.</p>
      </div>
    </header>

    <div class="focus-stage" :class="`focus-stage--${mode}`">
      <div class="timer-label">{{ mode === 'break' ? 'Break' : mode === 'paused' ? 'Paused' : 'Focus session' }}</div>
      <div class="timer-display" aria-label="25 minutes remaining">{{ String(minutes).padStart(2, '0') }}:00</div>
      <p class="planned-duration">Planned duration · {{ minutes }} minutes</p>

      <div v-if="mode === 'idle'" class="focus-setup">
        <div class="duration-control">
          <label for="focus-duration">Planned duration</label>
          <span>
            <input id="focus-duration" v-model.number="minutes" type="number" min="1" max="180" />
            minutes
          </span>
        </div>
        <label for="focus-task">Select an active task</label>
        <select id="focus-task" v-model="selectedTaskId">
          <option value="">Standalone activity</option>
          <option v-for="task in tasksStore.activeTasks" :key="task.id" :value="task.id">
            {{ task.title }}
          </option>
        </select>
        <div v-if="!selectedTaskId" class="field">
          <label for="focus-activity">Activity description</label>
          <input id="focus-activity" v-model="activity" maxlength="120" placeholder="Study JavaScript" />
        </div>
      </div>

      <div class="focus-subject">
        <span>Working on</span>
        <strong>{{ subject }}</strong>
        <small v-if="selectedTask?.course">{{ selectedTask.course }}</small>
      </div>

      <div class="focus-controls">
        <button
          v-if="mode === 'idle'"
          class="button button-primary button-large"
          type="button"
          data-test="start-focus"
          :disabled="!canStart"
          @click="startFocus"
        >
          <Play :size="20" fill="currentColor" /> Start focus
        </button>
        <template v-else-if="mode === 'running'">
          <button class="button button-primary button-large" type="button" @click="mode = 'paused'">
            <Pause :size="20" fill="currentColor" /> Pause
          </button>
          <button class="button button-secondary" type="button">Complete early</button>
          <button class="button button-ghost danger-text" type="button"><Square :size="17" /> Cancel</button>
        </template>
        <template v-else-if="mode === 'paused'">
          <button class="button button-primary button-large" type="button" @click="mode = 'running'">
            <Play :size="20" fill="currentColor" /> Resume
          </button>
          <button class="button button-secondary" type="button">Complete early</button>
          <button class="button button-ghost danger-text" type="button"><Square :size="17" /> Cancel</button>
        </template>
        <template v-else>
          <button class="button button-primary button-large" type="button" @click="mode = 'idle'">
            <RotateCcw :size="19" /> End break
          </button>
        </template>
      </div>
    </div>
  </section>
</template>
