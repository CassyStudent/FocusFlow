# FocusFlow

FocusFlow is an offline desktop application for students that combines lightweight task management with linked focus sessions and append-only Obsidian history.

## Current implementation status

The repository currently contains:

- Vue 3 + Vite + Tauri 2 desktop project setup;
- Pinia and Vue Router;
- the approved accessible light-pink application shell;
- onboarding plus Tasks, Focus, History and Settings routes;
- task creation, editing, completion, reopening and deletion;
- timestamp-based focus and break timers with pause, resume, early completion, cancellation and recovery;
- immutable session history with daily summaries and status filters;
- SQLite persistence through a narrow Rust/Tauri command boundary;
- append-only Obsidian Markdown output with a transactional outbox, retry and duplicate protection;
- configurable focus/break defaults, vault validation and pending-event recovery;
- Vitest frontend tests and Rust backend tests.

The timer-sound preference is persisted, but actual completion audio playback remains a documented follow-up.

## Prerequisites

- Node.js 22 or later
- npm 10 or later
- Rust stable toolchain
- Microsoft C++ Build Tools on Windows
- WebView2 runtime on Windows

## Install

```bash
npm install
```

## Run in a browser during UI development

```bash
npm run dev
```

## Run as a Tauri desktop application

```bash
npm run tauri dev
```

## Build the desktop application

```bash
npm run tauri build
```

## Test

```bash
npm test
```

## Design and architecture

- `docs/DESIGN_PLAN.md` contains the reference analysis, light-pink design system and exact Figma plans for the three required artifacts.
- `docs/TECHNICAL_DECISIONS.md` records the selected architecture and alternatives.

## Obsidian target structure

The completed application will append generated events under a user-selected folder:

```text
FocusFlow/
├── Tasks/
│   └── YYYY-MM-DD.md
└── Sessions/
    └── YYYY-MM-DD.md
```

Obsidian files are historical output. They are not the operational application database.
