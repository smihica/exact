surfaces: web, macos
agent: Codex
task: qualify generated data constructors

### Rough

The factory tests omitted the bake platform and expected a direct type alias.
Native entries now use a configured type so `Default` applies their composition.
The stateless case names web and checks its direct constructor; the reuse case
names macOS and checks its configured `Default` constructor.

### Lean in

Both source choices use the shared entry generator.

### Needed

none

### Checkpoints

Factory expectations now name the platform they qualify.
All 51 data integration tests pass; one measurement remains ignored.
