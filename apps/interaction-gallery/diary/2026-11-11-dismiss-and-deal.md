surfaces: web, linux
agent: Claude Code (Claude Fable 5.2)
task: finish two gestures when their motion ends, with a timer

The gallery's dismiss drag now closes the viewer and a thrown card deals the
next, each on a gated `after(150, …)` in the root (`docs/recipes/motion.md`).

### Rough

- A component cannot reset its own state when a prop changes. The root owns
  the deal, so the deck's pan outlives the card it threw. Keying the whole
  component by the deal (`for n in [dealt] key=n` around `CardDeck`) gave a
  fresh pan, and compiled and passed every test, but it also rebuilt the
  "Deal again" button under the press: `state focus` read `logical null`
  after a tap, where a button that stays keeps the focus. Nothing warned;
  I found it by asking. About 15 minutes. Instead the deck keys only its top
  card and keeps a `panDeal` beside the pan, so a card dealt since reads
  "0px 0px". A recipe line for "local state that belongs to one value of a
  prop", or a `check` finding for a press that removes the pressed control,
  would have helped.
- A plain `screenshot` straight after a release shows the release spring
  already at its end (the clip empty at clock 0 ms), on the web host. I read
  that as the photo never having been drawn until `screenshot … over 500
  every 50` showed the flight. A few minutes. The motion recipe names the
  film; it does not say a still shows the end state.
- `expect` reads the root's state and the tree, not a component's state or a
  node's translate. "The new top card starts at rest" is checked by hand
  (`layout deck-top`, the film), not in `app.test.contract`.
- The film shows the photo past its clip by 100 ms on both hosts. At 300 ms
  the viewer stood empty for about 200 of them, so the wait is 150. The
  recipe has no way to say "when it has left"; the length is a guess.
- A drag's `over 250` leaves the clock where it was and its `hold 300` moves
  it; the driver's usage says a hold moves the clock and nothing of `over`.
  It did not matter here, since the timer is armed at the release.
- Adding `app.test.contract` adds this app's tests to the web JS conformance
  run (`host/web-js/conform.mjs`), which needs the wasm build. I learned that
  from the script's header, not from the tests recipe, and did not run it.
- The first builds in a new worktree wrote 8 GB under `target/` (the
  compiler, the web build, the Linux host); the disk filled during the Linux
  host's and the build had to be run again.

### Lean in

- The recipe was the whole design: the action prop, the gated task and the
  call from the release handler compiled on the first try.
- A gate that compares the dismissed token with the open viewer's needs no
  cancelling code. Back, Previous, Next and Remove each change the token or
  close the viewer, and the timer goes with the gate.
- `screenshot … over 500 every 50` on a clock in my hands: one picture
  settles whether a card springs back.
- The same test file passed on the web and the Linux host unchanged.

### Needed

- timers: provided
- drag gestures: provided
- focus: provided

### Checkpoints

- install / first build: rough (the disk, above)
- web dev loop: smooth
- editing app.contract: smooth
- driving the app (`agent.mjs`, `--test`): smooth; rough for stills of motion (above)
- Linux build: smooth once the disk had room
