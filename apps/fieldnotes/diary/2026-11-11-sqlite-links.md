# SQLite links by use

## Rough

Every TypeScript storage session installed the bundled SQLite provider, even
when its app had no SQL grants. Rust storage named the same provider directly.

## Lean in

Fieldnotes already declares `sqlite.open` in both data modules. Those grants
now select `sqlite` in the existing native composition; its generated entry
registers the shared provider before constructing either module.

## Needed

Keep the provider in development and unknown compositions. Preserve the one
file-reservation registry shared by app storage and a native module's SQLite.

## Checkpoints

Storage executor tests pass, including transactions and persistent files.
Fieldnotes' plan-selected notebook fixture passes all 15 integration tests.
Its linked test executable keeps SQLite; the no-storage benchmark omits it.
