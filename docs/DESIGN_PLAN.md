# FocusFlow Visual Reference Analysis and Figma Plan

**Date:** 2026-09-27  
**Status:** Approved direction for the three required design artifacts

## Adapted visual principles

The supplied references are inspiration rather than templates. FocusFlow adopts their general hierarchy and spacing while retaining the approved PRD and an original light-pink identity.

### Desktop task-management reference

Use:

- a compact, persistent left sidebar;
- a large main work area with one clear page heading;
- simple horizontal task rows with restrained actions;
- generous whitespace and consistent alignment;
- rounded white or near-white surfaces against a tinted application background;
- a low-density desktop composition that is easy to scan.

Do not carry over:

- AI assistant panels;
- projects or list-management systems;
- priorities;
- impact scores, analytics or leaderboards;
- accounts, profile controls, subscriptions or upgrade cards;
- three-column density that reduces the task area.

FocusFlow should use two structural columns only: sidebar and current page. Dialogs and session details may appear as overlays or contained panels when needed.

### Pomodoro reference

Use:

- a very large countdown as the Focus screen's visual anchor;
- the selected task or standalone activity directly beside or beneath the timer;
- one unmistakable primary action;
- rounded, calm containers with minimal secondary content;
- strong separation between focus mode and break mode.

Do not carry over:

- the purple brand identity;
- the tilted mobile-device composition;
- long-break controls or other modes not approved in the PRD;
- decorative gamification;
- controls without explicit Pause, Resume, Complete Early and Cancel meanings.

## Surface choices

- **Tasks:** Operate surface. Actions and state visibility take priority over decoration.
- **Focus:** Monitor surface with an Operate secondary role. The timer dominates; controls remain explicit.
- **History:** Monitor surface. Daily progress and chronological records must be glanceable.
- **Settings/onboarding:** Configure surface. Validation and recovery status take priority over visual flourish.

## Light-pink design system

### Color tokens

| Token | Value | Role |
|---|---:|---|
| App background | `#FFF7FA` | Very light pink application canvas |
| Sidebar background | `#FDECF3` | Slightly stronger pink navigation area |
| Soft accent surface | `#FFF0F5` | Selected rows, timer panels and highlights |
| Card surface | `#FFFFFF` | Main cards and dialogs |
| Primary pink | `#C23B6C` | Primary actions and active navigation |
| Primary hover | `#A82E59` | Hover/pressed primary state |
| Pink border | `#F0CEDA` | Dividers, inputs and card outlines |
| Strong text | `#2F2429` | Main text and headings |
| Muted text | `#6F5F66` | Supporting labels and metadata |
| Destructive | `#B4233E` | Delete and destructive confirmations |
| Warning text | `#7A4A00` | Pending-vault warning text |
| Warning surface | `#FFF3D6` | Small semantic warning banner only |

Verified WCAG contrast examples:

- strong text on app background: 14.19:1;
- muted text on app background: 5.69:1;
- white text on primary pink: 5.08:1;
- destructive text on white: 6.46:1.

Pink remains the dominant identity. Amber is reserved for the small pending-event warning because it communicates a recoverable warning without being used as a brand color.

### Typography

Use a warm, modern system sans stack for the application and tabular numerals for timers.

- Page title: 30–34 px, 700 weight
- Section title: 20–22 px, 650–700 weight
- Task/session title: 15–16 px, 600 weight
- Body/control text: 14–16 px
- Supporting metadata: 13–14 px
- Main timer: 88–112 px, 700 weight, tabular numerals

### Shape and spacing

- Base spacing unit: 4 px; common gaps: 8, 12, 16, 24 and 32 px.
- Sidebar width: about 232 px.
- Main page padding: 32–40 px on standard desktop windows.
- Card radius: 16–20 px.
- Input/button radius: 10–12 px.
- Dialog radius: 20 px.
- Minimum control height: 44 px.
- Shadows: soft and shallow; borders carry most separation.
- Use icons only where they improve scanning. Always retain visible text for important actions.

## Artifact 1 — Tasks screen

### Figma frame

- Desktop frame: 1440 × 960.
- Two columns: 232 px sidebar and flexible main area.
- App background `#FFF7FA`; sidebar `#FDECF3`.

### Sidebar

Top:

- FocusFlow wordmark and a small original focus-ring mark.

Navigation:

1. Tasks — active state uses soft pink fill and primary-pink icon/text.
2. Focus
3. History
4. Settings

Bottom/status area:

- Active timer chip when relevant: `Focus · 18:42` or `Break · 03:21`.
- Pending warning chip when relevant: `2 Obsidian events pending` with a Retry affordance.

Do not include profile, account, upgrade, project or analytics sections.

### Main header

Left:

- `Tasks`
- short support text: `Plan your coursework and start focused work.`

Right:

- primary `Create task` button if quick creation is closed.

### Active/Completed control

Place directly beneath the header as a two-option segmented control. Use pink fill for the active segment and a white/outlined inactive segment. Include counts only if they represent actual task totals.

### Quick task creation

Use a white card above the task list:

- required Title field taking most of the row;
- optional Course/Subject field;
- primary Add Task button;
- inline validation below the relevant field;
- support Enter-to-create if it does not interfere with the optional field.

Keep it compact and title-first rather than presenting a large task form.

### Task rows

Use separate white rows with 14–16 px radius, thin pink border and 12–16 px vertical gaps.

Left area:

- task title;
- optional Course/Subject as a soft pink text badge;
- created date only if needed in the final detailed design.

Right area:

- `Focus` as the strongest row action;
- `Edit` and `Complete` as restrained secondary buttons;
- `Delete` as an icon/text action with a clear accessible label.

When a task is locked by the active session, disable Edit, Complete and Delete and show a concise reason on hover/focus or near the row.

### Required states on the same artifact

Include smaller side frames or component variants for:

- Completed view with Reopen and Delete;
- no active tasks;
- delete confirmation dialog;
- pending Obsidian warning;
- active timer indicator.

### Decisions this artifact records

- sidebar proportions and persistent status placement;
- quick-create placement;
- task-row density and action hierarchy;
- Active/Completed presentation;
- locked-task, empty and destructive states.

## Artifact 2 — Focus screen and timer states

### Figma frame

- Desktop frame: 1440 × 960 using the same sidebar and background.
- Center the focus composition in the available main area without turning it into a marketing-style hero.
- Keep supporting content narrow enough that the timer remains dominant.

### Idle composition

Top:

- page title `Focus`;
- short state label such as `Ready to focus?`.

Main timer card:

- soft-pink timer panel with white inner surface or restrained pink outline;
- large `25:00` timer using tabular numerals;
- `Planned duration` control placed above or beside the timer, not inside Settings-only navigation;
- subject selector directly beneath the timer:
  - active-task selector; or
  - standalone activity input;
- strong full-width or prominent `Start focus` button.

### State frames

Build a connected state wireflow with these frames:

1. Idle/task selection
2. Running — Pause primary; Complete Early and Cancel secondary
3. Paused — Resume primary; Complete Early and Cancel secondary
4. Complete Early confirmation
5. Cancel confirmation
6. Completed — actual duration, selected subject and Start Break
7. Break running
8. Break paused
9. Recovered completion message after reopening

### State rules

- Keep the selected task/activity immediately below the timer in every focus state.
- Show planned duration and actual/elapsed context without adding analytics.
- Use the same primary pink for Start, Resume and Start Break.
- Use a softer outlined treatment for Pause and Complete Early.
- Use destructive rose only for Cancel.
- Break mode should use a paler pink timer surface and explicit `Break` label rather than a new dominant color.
- Do not show social modes, streaks, achievements, leaderboards or long-break features.

### Decisions this artifact records

- timer scale and vertical hierarchy;
- subject visibility;
- button priority in each state;
- distinction between Completed Early and Cancelled;
- focus/break visual distinction;
- recovery feedback.

## Artifact 3 — History and Settings

Create two coordinated frames on one Figma page so the shared shell and visual system can be compared.

### History frame

Header:

- `History`
- support text explaining that sessions are read-only.

Daily summary:

- one wide, soft-pink summary card;
- `Today` label;
- two concise values: completed sessions and actual focused duration;
- no charts, trend arrows or invented analytics.

Filters:

- `All`, `Completed`, `Cancelled` segmented control.

Session list:

- group rows under Today, Yesterday or stored dates;
- each row shows status, task snapshot/activity, actual duration and end time;
- Completed Early appears as a secondary label;
- Cancelled uses restrained destructive text/icon treatment;
- selecting a row opens a read-only detail panel or dialog containing the PRD fields.

Include a zero-summary and empty-history variant.

### Settings frame

Organize settings as a single readable column of white cards rather than a dashboard.

Timer card:

- default focus duration;
- default break duration;
- note that changes affect future timers only.

Obsidian card:

- current vault path;
- Validate Vault;
- Change Vault;
- pending event count;
- Retry Now;
- writable/invalid/non-Obsidian warning states.

Sound card:

- one Timer sound toggle.

Application card:

- FocusFlow name and version only.

Include an inset for redirecting pending events when changing vaults. Do not include accounts, themes, subscriptions, integrations or advanced preferences.

### Decisions this artifact records

- summary prominence without an analytics dashboard;
- session-row density and immutable details;
- settings grouping and wording;
- recoverable Obsidian failure presentation;
- warning, empty and success states.

## Consistency requirements for all frames

- Apply the light-pink system to onboarding, dialogs, warnings, empty states and every timer state.
- Preserve dark readable text and visible focus rings.
- Do not rely on pink alone to communicate status; pair color with labels and icons.
- Keep destructive actions visually separate from the primary pink action.
- Use the same sidebar, page padding, radii, typography and control heights throughout.
- Every UI element must map to the approved PRD.
