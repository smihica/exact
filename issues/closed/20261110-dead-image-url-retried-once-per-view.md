# A failing image URL shown by N images is retried 6N times, and each image's `error` waits for all of them

**Status:** Closed
**Resolution:** A shared download retry counts for every view joining it on screen, so repeated images exhaust one retry ladder.
**Systems:** host/linux/src/image.rs (remote images on the painting host: Linux, Windows, Android)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A picture on screen whose download fails in a way asking again may mend is asked for again
"six times a showing, about a minute in all; once a showing has spent them, its `error` is
sent and it is not asked for again (a dead URL never polls)"
(`host/linux/src/image.rs:170` to `:173`; LLP 1011 §4, "Retries").

The count is kept per view (`View::retries`, `image.rs:86` to `:91`), but the views of one
URL share one download. When one view's time comes it evicts the failed source and counts a
retry (`image.rs:606` to `:616`). Every other view of that URL then finds its source gone
and joins the new download "with no retry of its own counted or due" (`image.rs:601` to
`:605`). When that download fails too, the views that joined are still at their old count
and at an earlier step of the ladder, so one of them asks next. Each step of the ladder is
therefore taken once per view: N views of a URL make 1 + 6N requests over N ladders, and no
view's `error` is sent until its own six are spent.

A list is the common case: a sender's avatar beside each of their messages, a product
thumbnail repeated in a cart. With the network down, a URL in 20 visible rows is asked 121
times over about 20 minutes, and the app's `error` fallback (initials, a placeholder)
appears only then.

## The failure

A server that answers 503 to everything, and

```
column width=300 height=300
  image "http://127.0.0.1:PORT/dead/a.png" width=50 height=50 error=failed
```

once, then three times in the column. The test build's ladder is 10, 20, 40, 80, 160 and
300 ms (610 ms in all).

```
1 image of one dead URL:  7 requests,  1 error,  0.76 s until the last error
3 images of one dead URL: 19 requests, 3 errors, 2.11 s until the last error
```

Expected for three images: 7 requests and all three errors after one ladder.

## Verification

A temporary test in `host/linux/src/presenter/images_tests.rs`, using that file's
`scripted_server` on 127.0.0.1, booted each Contract and counted the server's requests until
every `error` had run: the two lines above. For comparison, a page of 1, 3 and 20 `<img>` of
one URL served by a local server answering 503, in Chromium 156: the server was asked 1, 2
and 2 times, every `error` event fired within 11 ms, and nothing more was asked in the next
3 s. Not run: the production ladder (61 s); the Apple host's `RasterDownloads`.

## Direction

Count a retry against the URL, not the view: a view that joins another's retry
(`image.rs:601` to `:605`) takes that view's count and waits the same step, or the count and
the next time live on the source beside `Backend::transient`.

## Related

`a_failure_that_asking_again_may_mend_errs_once_its_showings_retries_are_spent`
(`presenter/images_tests.rs:613`) covers one image.
`issues/20261110-large-pictures-decode-at-half-or-quarter-resolution.md` and
`issues/20261110-image-alt-text-skips-the-cell-policy.md` are other image findings. Nothing
in `QUEUE.md` or `docs/agent-pitfalls.md` names a URL shown by several images.
