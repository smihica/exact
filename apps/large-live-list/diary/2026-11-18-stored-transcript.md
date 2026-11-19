surfaces: web, macos
agent: Codex
task: build and measure a stored transcript

### Rough

A socket mapper cannot use storage. It stages frames and returns a sequence
number so a separate mutation can persist the frames before refreshing the list.
The sequence number alone cannot carry the frames because stream answers can
coalesce. On the first reconnect drive, sending from the storage queue's
callback also failed because it ran outside the socket answer's turn. Moving
the send into that answer's async continuation fixed replay. This took about
five minutes to reproduce and trace.

The first live Mac runs found the newest row in the tree while it was still
below the viewport. Using the short rows' minimum height as the list's
estimate and scrolling to the end after the first answer put the arrival in
the viewport. Checking geometry and rerunning took about three minutes.

Authored `fail fetch` cannot close a WebSocket. The loopback fixture now
closes connections and drops acknowledgements through a test-only HTTP
control. The web driver also intercepts SQLite worker requests to hold a
write or read and refuse an outbox lookup at a known point.

Passing the revision after `with` kept the window's identity stable. The
first relaunch still waited for SQLite because the closed thread's gate
prevented the bake from seeing a storage read. An empty read on the first
launch lets the bake identify the reader. The app persists whether the
thread was open so it can show the matching latest window on relaunch.

A change task cannot clear the edge request in its own trigger. A one-shot
task can consume the request after the matching window settles. The paging
test uses wheel input on both web and macOS.

The socket mapper receives a terminal `error` when the host refuses a
burst. Returning a status for that event left `failed(feed)` false. Throwing
from the mapper lets the app commit the staged frames and recover. Holding
browser frame delivery in the test reproduced the refusal reliably.

The hourly lane discovers the Bun test for a crate-free app. The test starts
its own loopback server and runs the authored suites against the builds it
makes. The web driver checks build settings against its process environment,
so a drive needs the same socket setting as its build.

### Lean in

A resource answer can carry both edge cursors beside the keyed rows.

### Needed

- storage — provided
- WebSocket — provided
- outbox — by hand

### Checkpoints

- install / first build — smooth
- editing app.contract / app.ts — rough, see the socket mapper and list estimate
- macOS build — smooth
- driving the app — rough, see the socket and SQLite test controls

### Baseline

Measured at `6e5220c64` on a Mac Studio (Mac17,15, Apple M5 Ultra, 256 GiB)
running macOS 27.2.1 (26B131). Each row reports minimum / median / maximum
over five runs after a warm-up. Each run started a new host and scratch store,
seeded one conversation, opened it and received one loopback socket message.
The open measurements exclude seeding.

| Host and build | Seed messages | Open (ms) | Socket arrival to paint (ms) | Process RSS (MiB) |
| --- | ---: | ---: | ---: | ---: |
| macOS `host-dev`, `6e5220c64` | 1,000 | 118.90 / 131.04 / 186.07 | Unavailable | 131.52 / 132.23 / 132.50 |
| macOS `host-dev`, `6e5220c64` | 10,000 | 120.45 / 144.97 / 221.66 | Unavailable | 133.06 / 133.61 / 133.80 |
| macOS `host-dev`, `6e5220c64` | 100,000 | 86.22 / 147.94 / 166.58 | Unavailable | 134.38 / 134.89 / 135.38 |
| Web JS, headless Chrome 158.0.8127.37, `6e5220c64` | 1,000 | 37.20 / 48.10 / 54.30 | 114.30 / 118.20 / 132.20 | 971.14 / 973.45 / 975.78 |
| Web JS, headless Chrome 158.0.8127.37, `6e5220c64` | 10,000 | 35.00 / 43.30 / 48.50 | 116.70 / 116.80 / 132.40 | 984.11 / 985.19 / 985.92 |
| Web JS, headless Chrome 158.0.8127.37, `6e5220c64` | 100,000 | 34.30 / 40.50 / 71.10 | 69.10 / 122.50 / 131.30 | 1039.59 / 1042.73 / 1047.88 |

Mac open time runs from the driver tap to the new row in the tree. The web
uses the click and socket message events with the browser's Element Timing
paint for the target row. The macOS driver verifies the row in a screenshot
after it lands and has no first-presentation timestamp for that row. RSS is
the Mac app process or the sum of the browser and its descendant processes
after the arrival. The browser sum can count shared pages more than once.
Source CPU time was unavailable on both hosts.

On macOS, loopback send to the first driver observation of the arrival in the
tree took 22.09 / 32.30 / 41.13 ms at 1,000 messages, 23.03 / 34.67 / 36.74 ms
at 10,000, and 33.78 / 36.12 / 40.44 ms at 100,000. The driver confirmed the
row in the viewport and captured it after each observation.
