PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 120),
  course TEXT CHECK(course IS NULL OR length(course) <= 60),
  status TEXT NOT NULL CHECK(status IN ('active', 'completed')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  activity TEXT,
  task_title_snapshot TEXT,
  course_snapshot TEXT,
  planned_seconds INTEGER NOT NULL CHECK(planned_seconds > 0),
  actual_seconds INTEGER NOT NULL CHECK(actual_seconds >= 0),
  status TEXT NOT NULL CHECK(status IN ('completed', 'cancelled')),
  completion_type TEXT CHECK(completion_type IN ('timer_finished', 'completed_early')),
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  event_offset TEXT NOT NULL,
  local_date TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  focus_minutes INTEGER NOT NULL CHECK(focus_minutes > 0),
  break_minutes INTEGER NOT NULL CHECK(break_minutes > 0),
  sound_enabled INTEGER NOT NULL CHECK(sound_enabled IN (0, 1)),
  vault_path TEXT
);
INSERT OR IGNORE INTO settings(singleton, focus_minutes, break_minutes, sound_enabled, vault_path)
VALUES (1, 25, 5, 1, NULL);

CREATE TABLE IF NOT EXISTS active_timer (
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  id TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('focus', 'break')),
  state TEXT NOT NULL CHECK(state IN ('running', 'paused')),
  task_id TEXT,
  activity TEXT,
  task_title_snapshot TEXT,
  course_snapshot TEXT,
  planned_seconds INTEGER NOT NULL CHECK(planned_seconds > 0),
  accumulated_seconds INTEGER NOT NULL CHECK(accumulated_seconds >= 0),
  remaining_seconds INTEGER NOT NULL CHECK(remaining_seconds >= 0),
  started_at TEXT NOT NULL,
  last_resumed_at TEXT,
  state_changed_at TEXT NOT NULL,
  event_offset TEXT NOT NULL,
  local_date TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS used_ids (
  kind TEXT NOT NULL CHECK(kind IN ('task', 'timer', 'event')),
  id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY(kind, id)
);

CREATE TABLE IF NOT EXISTS outbox (
  event_id TEXT PRIMARY KEY,
  category TEXT NOT NULL CHECK(category IN ('tasks', 'sessions')),
  local_date TEXT NOT NULL,
  markdown_body TEXT NOT NULL,
  created_at TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('pending', 'delivered')),
  delivered_at TEXT,
  last_error TEXT
);
CREATE INDEX IF NOT EXISTS outbox_pending_chronological ON outbox(status, created_at, event_id);
CREATE INDEX IF NOT EXISTS sessions_history_chronological ON sessions(ended_at DESC, id DESC);
