# 0044. A bounded daily log file is the operators' evidence

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #311, a9d3dbd); reports 2026-08-21 (PR #268, #271); diag log 2026-08-09 (#163) |
| **Area** | Platform |
| **Evidence** | `src/main/file-log.js`; `src/main/index.js:95-122, 292-306, 372-440, 581-680`; `test/file-log.test.js`; `renderer.js:1313` (`__health`); labs-videowall-config `startup/README.md` |
| **egui verdict** | Keep |
| **egui status** | Not started |

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
interface with the people reading them. `tracing` with a custom appender
implementing the same limits; a panic hook in place of process-death events;
a keyboard or CLI health dump in place of `__health()`. Not started: the
prototype logs to stderr only, though its prefixes (`[health]`, `[capture]`,
`[render]`, `[updater]`) are already one flat line per event.
