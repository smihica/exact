# Speech synthesis on a phone, by a person (LLP 1133.012 §10)

The drives a phone needs, done with the Markdown reader (`apps/markdown`)
built as in production: no agent, the package bound. Open a document with
a few paragraphs, such as `apps/markdown/assets/listen-fixture.md`.

- iPhone: `bun host/apple/build.mjs --ios --device markdown-apple --run`

On Android, use an app that speaks with each of the three `audioSessionType`
values for steps 3 and 4. Step 4 was verified with YouTube Music on a POCO
X6 Pro running Android 14. The call in step 3 still needs a device check.

Each step says what to do and what should happen.

1. **Listen.** Tap Listen. The document is read aloud from the top, one
   paragraph after another, and the word being spoken is marked. Pause
   holds the mark and the voice, and Resume goes on from there. When the
   last paragraph ends, "Finished" shows.
2. **The screen locked** (iPhone). Tap Listen and lock the phone. Reading
   goes on. Unlock: the mark is at the word being spoken. Lock-screen
   controls are not required: Now Playing for a speech `playback` queue
   remains deferred (`rules/DEFERRED.md`, Speech).
3. **A call.** While reading, call the phone from another one. Reading
   pauses while the call rings. End the call: reading resumes, or Resume
   resumes it.
4. **Another app's audio.** On iPhone, play a podcast and tap Listen in
   the reader. The podcast pauses. Tap Stop and check that the podcast resumes.
   On Android, play music in another app before each run. With `transient`,
   the music ducks during speech and returns to full volume afterward. With
   `transient-solo`, it pauses and can resume afterward. With `playback`, it
   stays paused after speech ends until the listener starts it again.
5. **A silent switch** (iPhone). With the ring switch on silent, Listen
   still speaks (`playback`, not `ambient`).
6. **Leaving.** Tap Listen, then open another document. Reading stops.
