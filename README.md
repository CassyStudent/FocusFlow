# FocusFlow

FocusFlow is an offline desktop application for students that combines lightweight task management with linked focus sessions and append-only Obsidian history.

## Current implementation status

The repository currently contains:

- Vue 3 + Vite + Tauri 2 project setup;
- Pinia and Vue Router;
- the approved light-pink application shell;
- Tasks, Focus, History and Settings routes;
- a working first Tasks vertical slice with create, edit, complete, reopen and delete interactions;
- title and Course/Subject validation;
- browser-development persistence for the initial Tasks slice;
- initial Focus, History and Settings visual states;
- Vitest domain and component tests;
- Tauri SQL and dialog plugins prepared for the durable SQLite/vault implementation.

SQLite persistence, timer recovery, Obsidian append/outbox behavior and complete focus-session state transitions are the next implementation phases.

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
