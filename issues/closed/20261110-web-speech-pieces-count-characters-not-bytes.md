# The web's split of a long speech result cuts at 16,384 characters, not 16 KB, and can add an empty result

**Status:** Closed
**Resolution:** Web speech pieces stay within 16,384 UTF-8 bytes, preserve code points and omit empty split tails.
**Systems:** packages/speech-recognition/web/index.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

A final result's text reaches the app "in pieces of at most 16 KB, split at a word boundary"
(LLP 1133.012 D7, whose bound "is in bytes"; the comment on `pieces`). At `dd6f5dcd` the web's
`pieces` (`packages/speech-recognition/web/index.js:36-47`) tests the text's UTF-8 length but
cuts by UTF-16 index:

```js
while (new TextEncoder().encode(rest).length > SPLIT) {
  let cut = rest.lastIndexOf(' ', SPLIT);
  if (cut <= 0) cut = SPLIT;
  out.push(rest.slice(0, cut));
  rest = rest.slice(cut).trimStart();
}
out.push(rest);
```

For text whose characters take more than one byte, 16,384 characters is up to three times the
bound. When the text is shorter than 16,384 characters but longer than 16 KB, the first piece
is the whole text and the loop leaves `rest` empty, which is then pushed: `final` numbers it as
a result of its own (`:86-91`), so `finalCount` is one too many and the transcript gains a
result with no text. Apple cuts at a UTF-8 offset (`apple/SpeechRecognition.swift:391-401`) and
Android shrinks the cut until its bytes fit (`android/SpeechRecognition.kt:425-437`).

**The failure.**

```js
import { pieces } from './packages/speech-recognition/web/index.js';
pieces('あ'.repeat(6000)).map(p => new TextEncoder().encode(p).length)   // [18000, 0]
```

Expected, as Apple and Android give: two pieces, each at most 16,384 bytes, and no empty one.

**Verification.** Run under Bun 1.4.2, the exported function called on four texts:

```
English, 40 KB: 40960 bytes in -> pieces of 16384, 16384, 8190 bytes
Japanese, 6,000 characters, no spaces: 18000 bytes in -> pieces of 18000, 0 bytes
Japanese, 20,000 characters, no spaces: 60000 bytes in -> pieces of 49152, 10848 bytes
Japanese, 6,000 characters, one space near the end: 17998 bytes in -> pieces of 17970, 27 bytes
```

Not run: a browser recognizer producing a final that long. Browsers end a final at a pause, so
this needs a long unbroken utterance in a script without spaces.

**Direction.** Find the cut by encoded length (shrink the slice until its bytes fit, as
`SpeechRecognition.kt` does, or walk the encoder's output), and do not push an empty rest.

**Related.** None found.
