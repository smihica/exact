surfaces: linux, web
agent: Claude Code (Claude Opus 5.5)
task: keep a stored file when reading it fails

### Rough
Shelf's `load` treated every failed read except `bake` as a first launch, so
the next edit saved the seed over the library. Messages, Agents, Weatherlight
and Recorder did the same. The recipes and starter patterns already branch on
`ENOENT`; the apps predated them.
`bun test apps/shelf` failed before reaching any test: the build generates
`@exact/nfc`'s module, and Bun read the package's `exact.json` instead. The
test now mocks the package, as Agents' and Photo Editor's tests do.

### Lean in
`e.code` names the failure on every host (`ENOENT`, `denied`, `EBUSY`, `full`,
`failed`), so the fix is one comparison in each app.

### Needed
- files: provided

### Checkpoints
- load: a read that fails other than `ENOENT` leaves the library not ready and
  every edit answers `ok: false` without writing; the next successful read
  finds the stored books.
