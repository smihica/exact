# Recipe for a large live list

Use a bounded window when rows change individually and the history exceeds
one answer. The [sample](../../apps/large-live-list/) seeds 100,000 messages.

Grant `sqlite.open` for both database paths. Socket mode uses
`app:/data/messages.db` and fixture mode uses `app:/data/fixture.db`.
Index `(conversation, sort_order, id)` and page by that tuple. For an anchor,
read up to 100 rows through it and up to 100 strictly newer, plus a probe on
each side. For an empty cursor, read the newest 200 with one older probe.
[Schema and queries](../../apps/large-live-list/app.ts)

Pass the revision after `with` in `loadWindow(conversation, cursor) with
revision` to ask again under the same kept identity. While the answer is
pending, keep the edge request with the visible conversation and cursor.
Apply it only when that window settles. After paging, restore the overlapping
row by key. Follow the end only at the latest cursor.
[Transcript component](../../apps/large-live-list/app.contract)

Stage frames outside the stream answer. On open, ask for replay after the
committed sequence. Commit the contiguous rows, unread count, and sequence
in one transaction. At a gap, discard the rest of the inbox. On a gap or
feed failure, finish the commit before reconnecting. Show Retry if an
automatic attempt fails without advancing the sequence. Refresh the window
before looking up the next send. Count unread once per row, using the
sender ID carried in live frames and replay.
[Socket sources](../../apps/large-live-list/app.ts)

Capture the draft's edit counter at submit. Clear the draft after the
outbox write commits if the counter still matches. Send the oldest item
first and keep its ID for retries. Apply the acknowledgement and retire
the outbox row in the same transaction, even if replay already added the
message. [Loopback server](../../apps/large-live-list/socket.mjs)

## Limits

- Each host queues up to 256 undelivered socket frames. A burst can
  close the feed and trigger replay from the committed sequence.
- If the socket stays open and an acknowledgement never arrives, later
  sends wait until reconnect.
- The scrollbar spans the resident window. The app owns cursor overlap
  and reader anchoring.
- macOS can show a kept window on the first frame when the conversation
  and cursor match. The web JS target loads the first rows from SQLite.
- Web SQLite imports the file on open and exports it to IndexedDB after
  writes. Values, bind parameters and results have a 16 MiB adapter
  limit. The file is subject to browser quota.
- Use `drop stream` to inject a WebSocket network error. The loopback
  server's controls exercise peer closes, missing acknowledgements and replay gaps.
  [Test setup](../../apps/large-live-list/README.md)
