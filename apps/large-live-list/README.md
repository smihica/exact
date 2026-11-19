# Large Live List

Prepare a conversation and open it. Scroll to either edge to page through
the transcript in 200-row windows. **Latest** returns to the tail.

Without `MESSAGE_SOCKET`, the app runs in fixture mode. **Inject arrival**
adds a message and increases unread. Use **Go offline**, send a message, then
use **Go online** to acknowledge queued sends. The outbox survives relaunch.
Each mode has its own database. Prepare the conversation in the mode you use.

## Socket mode

From the checkout root, start the loopback server in a separate terminal.

```sh
bun apps/large-live-list/socket.mjs
```

Set the printed URL in the shell where you build and run the app.

```sh
export MESSAGE_SOCKET=ws://127.0.0.1:48732
bun host/web/build.mjs large-live-list
bun host/apple/build.mjs large-live-list
```

Open a conversation and choose **Connect socket** to receive its welcome
message. Disconnect, send a message, and connect again to send it from the
outbox. The server retains each conversation's log and deduplicates sends by
message ID for the life of the process. After restarting the server, use a
fresh socket store. A production server needs to persist the log and message IDs.

## Tests

From the checkout root, use the pinned Bun.

```sh
bun test --isolate ./apps/large-live-list/web.test.mjs
```

The hourly lane runs this file. It starts a test server on a free loopback
port, builds both modes, and runs `app.test.contract` against the fixture
build and `socket.test.contract` against the socket build. Both authored
suites run on web and, on a Mac, macOS. The browser tests also hold SQLite
requests and socket frames, check replay and unread across clients, and
switch modes with a queued send.

Use `-t 'web socket Contract suite'` to run just that authored suite.
Substitute `macos` for the host or `fixture` for the build mode.

See the [recipe](../../docs/recipes/large-live-list.md) for storage and paging.
