# 0044. A bounded daily log file is the operators' evidence

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #311, a9d3dbd); reports 2026-08-21 (PR #268, #271); diag log 2026-08-09 (#163) |
| **Area** | Platform |
| **Evidence** | `src/main/file-log.js`; `src/main/index.js:95-122, 292-306, 372-440, 581-680`; `test/file-log.test.js`; `renderer.js:1313` (`__health`); labs-videowall-config `startup/README.md` |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

On 2026-10-01 a card came up black at 12:12:27 and someone restarted the app
at 12:22:35; there was no record of what the renderer had seen. The wall's
operators now read `[Health]` lines from the log to diagnose black inputs.

## Decision

- Everything either process prints goes to
  `<userData>/logs/input-viewer-YYYY-MM-DD.log`: **7 days** kept (pruned by
  the date in the file name), **20 MB/day**, **600 lines/min** (with a
  "dropped N" line), lines cut at 2000 chars, local-time days.
- **Synchronous appends** -- "a crash or a hard power-off is exactly when
  the last lines matter". A failed write disables the log rather than throw.
- Startup banner: version, OS, arch, runtime version, **OS uptime** ("how
  long after the machine came up did the app open the cards"), pid, log
  directory. Process deaths are logged.
- Log flat strings: objects arrive as `[object Object]`. Keep lines ASCII
  for Windows tools.
- Per-side health transitions and a 10-minute summary; agreed stream
  formats; crop changes with the measured ratio.
- Files that run for months are bounded by content: `gpu-report.txt`
  (overwritten per launch, <= 200 lines), `fps-report.txt` (overwritten every
  ~60 s), the detection diagnostic (capped at 2 MiB, newest half kept).
- `__health()` in DevTools prints the current state.

## Consequences

The rate limit is shared, so a busy renderer can crowd out main's lines.
The diagnostic log is labelled "temporary" and has lived since 2026-08.
Whether DevTools, and so `__health()`, is reachable in a packaged build is
not documented.

## Porting to egui

**Keep** the path, file naming, bounds and line formats -- they are an
interface with the people reading them.

**Done** (`input_viewer_egui/src/filelog.rs`): a port of `file-log.js` with
its tests ported case for case.

- **Where:** `<config>/Input Viewer (egui)/logs/input-viewer-YYYY-MM-DD.log`,
  next to `settings.json`. Local-time days.
- **Same bounds:** 7 days by file-name date, 20 MB/day with one "cap
  reached" line, 600 lines/min with a "dropped N" line, 2000 characters per
  line.
- **Same behaviour:** synchronous appends; a log that cannot be written
  disables itself instead of failing.
- **Every line goes to stderr and the file.** Levels match Electron's
  `console.log`/`warn`/`error`, and the tags match where an equivalent exists
  (`[Health]`, `[AutoUpdater]`, `[TestFlags]`, `[Settings]`).
- **`[Health]` lines in `describeHealth()`'s exact format, one per half
  showing the device:**
  `[Health] right (Mock Input 2): ok -> dark 14.4fps luma=10+/-0 1280x720 reopens=0`.
  Also written: the reopen line, the "opened" line, the 10-minute summary,
  and `monitoring every 2000ms` (or `Not started: --no-signal ...`).
- **Startup banner** with OS uptime. A panic hook puts the panic message in
  the file.
- **GPU line** in place of `gpu-report.txt`: `[App] GPU: <vendor> /
  <renderer> / OpenGL <version>`. That answers the question the report
  existed for: is the wall rendering in software?

Differences:

- The source tag is `[app]` (one process) rather than `[main]` or
  `[renderer]`.
- The reopen line's id drops the `camera-` prefix before taking eight
  characters, since `camera-0` would say nothing.

Not ported:

- `fps-report.txt` -- `--hud` and its `[Render]` lines cover it.
- The detection diagnostic -- there is no reference detection yet
  ([0014](0014-detect-no-signal-against-references.md)).
- `__health()` -- there are no DevTools; a key or CLI dump would replace it.

Mock and review runs log too, as Electron's always does. Only settings are
protected from test mode.
