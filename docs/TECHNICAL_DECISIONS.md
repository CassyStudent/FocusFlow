# FocusFlow Technical Decisions

**Date:** 2026-09-27  
**Status:** Initial architecture approved for implementation

## 1. Framework

**Chosen:** Vue 3 with the Composition API and JavaScript, built with Vite.

**Why:** The developer already knows Vue 3, Composition API and Vite. JavaScript keeps the project explainable within the school assignment while domain boundaries and tests provide structure.

**Alternative considered:** Vue with TypeScript.

**Why not chosen:** TypeScript would improve compile-time checking, but it would add a second learning concern that is not required by the assignment. It can be introduced later if the developer becomes comfortable with it.

## 2. Desktop framework

**Chosen:** Tauri 2.

**Why:** FocusFlow needs a real offline desktop window, access to a user-selected local folder, native file dialogs and local persistence. Tauri allows the existing Vue/Vite skills to be used for UI while exposing a small Rust backend for filesystem operations.

**Alternative considered:** Electron.

**Why not chosen:** Electron would be viable, but it bundles a larger runtime and would make the desktop layer heavier than necessary for this focused student project.

## 3. Local storage

**Chosen:** SQLite through the official Tauri SQL plugin.

**Why:** Tasks, immutable session history, settings, active timer recovery and the event outbox have relationships and ordering requirements. SQLite supports durable local transactions and lets operational changes and outbox creation be committed together.

**Alternative considered:** JSON or browser localStorage.

**Why not chosen:** A single document is initially simpler but becomes fragile for transactional updates, chronological outbox delivery and future schema changes. Obsidian Markdown remains an output, not the operational database.

## 4. State management

**Chosen:** Pinia.

**Why:** Pinia will hold current UI/application state and call repository services. SQLite remains authoritative; Pinia is not used as permanent storage.

**Alternative considered:** Component-local state only.

**Why not chosen:** Tasks, the persistent timer indicator, history totals and pending-event warnings cross route boundaries.

## 5. Routing

**Chosen:** Vue Router with four routes: Tasks, Focus, History and Settings.

**Why:** These match the approved navigation and keep screen responsibilities explicit.

## 6. Filesystem access and Obsidian

**Chosen:** Native folder selection through the official Tauri dialog plugin and narrow Rust commands for vault validation, directory creation, Event-ID duplicate checking and append-only writes.

**Why:** The Rust boundary can validate paths and perform predictable append operations without exposing unrestricted filesystem access to the entire UI.

**Alternative considered:** General frontend filesystem-plugin access.

**Why not chosen:** Broad frontend access is harder to constrain and explain. The application only needs a small set of deliberate vault operations.

## 7. Obsidian structure

**Chosen:** Daily append-only Markdown files:

```text
FocusFlow/
├── Tasks/
│   └── YYYY-MM-DD.md
└── Sessions/
    └── YYYY-MM-DD.md
```

Each outbox event stores its rendered Markdown payload, target kind, target local date and unique Event ID. A retry checks for the Event ID before appending.

## 8. Timer architecture

**Chosen:** Timestamp-based state machine rather than trusting a repeating interval as the source of truth.

Persisted active-timer data includes:

- timer kind;
- session identity and subject snapshot;
- planned duration;
- accumulated focused seconds;
- last-resumed timestamp;
- current state;
- remaining duration needed for paused recovery.

A UI interval refreshes the display, but elapsed time is derived from timestamps. This supports closing, reopening, sleep and hibernation recovery.

## 9. Time and timezone handling

**Chosen:** Store canonical UTC instants plus the numeric UTC offset and local date captured at each event.

**Why:** UTC provides stable comparison while the stored offset/local date preserves what the user saw and determines the daily Obsidian file. Historical offsets are never recalculated.

Use integer seconds for durations. Daily totals sum seconds before formatting.

## 10. Error handling

- SQLite/local persistence failure is blocking and the UI must not report success.
- Obsidian failure is recoverable: the local transaction remains committed and its event stays in the durable outbox.
- Input errors are inline and retain form values.
- Invalid timer recovery state produces an explicit recovery error rather than an invented completion.

## 11. Testing

**Chosen:** Vitest and Vue Test Utils for domain, store and component behavior; Rust unit tests for Markdown/path logic.

Tests should target external behavior:

- task validation and ordering;
- state transitions and duration calculation;
- daily-summary aggregation;
- event Markdown rendering;
- outbox ordering and duplicate handling;
- UI actions and disabled states.

## 12. Styling

**Chosen:** SCSS with shared design tokens and component-level styles.

The implementation uses the light-pink palette defined in `docs/DESIGN_PLAN.md`. Important text/control combinations must meet WCAG AA contrast. Status is communicated with wording/icons as well as color.

## 13. Git workflow

Use one repository and small meaningful commits. Initial sequence:

1. project and Tauri scaffold;
2. technical/design documentation;
3. application shell and theme;
4. task domain and persistence;
5. focus timer;
6. history and summary;
7. Obsidian outbox;
8. settings/onboarding;
9. packaging, tests and documentation.
