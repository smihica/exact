# The iOS hello app is about 2.1 MB over its 8 MB size target

**Status:** Closed
**Resolution:** The size row measures 7,871,008 B at f607bac14 (12,489,072 B at 02639b8ea), under the 8,000,000 B target, with each Apple family linked by use (LLP 1047.001 §9); its ceiling is 7,970,000 B.
**Systems:** host/apple (iOS binary size), runner (native module seam), scripts/metrics.mjs
**Severity:** P2
**Author:** Claude (Opus 5.5), async lane triage
**Date:** 2026-11-13

`bun scripts/metrics.mjs --long` measures the hello app's stripped iOS simulator executable over an empty UIKit
app's (LLP 1047.001 D8) and gates it at `IOS_HELLO_BYTES`. The size row's target is 8 MB, 8,000,000 B (Charlie,
2026-11-17: "trying to get to 8MB is maybe worth it"). Main at f607bac14, measured on an M5 Max with Xcode 27.2:

| row | measured | ceiling | target |
|---|---:|---:|---:|
| optimized for size (the embed default) | 7,871,008 B | 7,970,000 B | 8,000,000 B |
| balanced (the iOS and tvOS app default) | 11,640,832 B | 11,740,000 B | |
| optimized for speed (the Mac default) | 12,795,968 B | 12,900,000 B | |

The ceilings are a tripwire at main's size plus about 100 KB, not an accepted size. A row over its ceiling says how
far over in 100 KB steps (`VIOLATION by 0.2 MB`), and the async lane's failure carries the step, so each further
step up is a new failure the lane attributes to the commit that took it, and a step down is not
(`scripts/async-bisect.mjs`, `newFailures` and `holds`). The size build was 12,489,072 B when the cuts began
(02639b8ea) and is under the target now; the Progress table lists each cut. Byte counts differ between Xcode
versions, so measure with the lane's.

## How to reproduce

`bun scripts/metrics.mjs --long` prints the three `iOS: hello app` rows. To measure them alone, run what the
`ios-size` step runs: for each of `speed`, `size` and `balanced`, `EXACT_UPDATE_TRUST=production
EXACT_UPDATE_GENESIS=1 bun host/apple/build.mjs hello-apple --ios --no-install --optimize <mode>`, then `xcrun
strip -o <copy>` the executable `appleArtifacts(hello, { destination: 'ios-simulator', trust: 'production',
optimize })` names, and subtract the stripped `host/apple/ios/floor.swift` built with `swiftc -Osize` (53,680 B).
The dSYM beside the executable attributes the bytes by symbol.

## How it grew

Speed row, bytes over the floor, on one Mac with Xcode 27.2 (commits before 2026-11-01 are the private
repository's):

| commit | date | speed | Rust text | Swift text |
|---|---|---:|---:|---:|
| e6caf40bf (the October ceiling) | 10-08 | 8,495,136 | 3.17 MB | 3.80 MB |
| c4d5e135bb | 10-16 | 10,252,832 | 3.76 MB | 4.62 MB |
| 636ff1263c | 10-17 | 11,035,728 | 4.06 MB | 4.95 MB |
| bffeda03f6 | 10-22 | 13,201,360 | 4.84 MB | 5.97 MB |
| f2447c136d | 10-23 | 13,568,704 | 4.98 MB | 6.16 MB |
| 8388cd0a48 | 10-28 | 16,290,928 | 6.55 MB | 7.07 MB |
| 029cd282d3 | 11-01 | 16,813,472 | 6.57 MB | 7.45 MB |
| 2a2f95a00 | 11-13 | 18,806,368 | 7.35 MB | 8.53 MB |

About 0.3 MB a day, with no single jump. Once a row was over its ceiling, further growth was the same failure, so
the lane attributed none of it; the stepped failure above is the fix for that.

Cut so far: the launch preloads (LLP 1162.000) are held as a `dyn LaunchWork` that only `IoLinks` makes (−82 KB
speed, −33 KB size), and a production composition compiles none of the Swift agent (−993 KB speed, −783 KB size;
its development builds keep it, and the production link proof in `metrics.mjs` checks both).

## Where the size build's bytes are

From the link map of the size-mode production hello at 02639b8ea (12,489,072 B over the floor), which sizes each
atom itself:

| object | bytes |
|---|---:|
| ExactKit.o | 6,189,809 |
| the Rust archive | 4,674,338 |
| ExactCore.o | 970,771 |
| linker-synthesized | 241,833 |
| ExactIOS main.o | 54,437 |
| everything else (system stubs, other small objects) | 60,089 |

Swift is about 7.2 MB of the executable and Rust about 4.7 MB. Rust families below come from the map's atoms by
crate and module path, a generic from `core` or `alloc` charged to the crate it is instantiated over. Swift bytes
come by source file from the dSYM's line table; 5.85 MB of Swift is attributed to a file, and the rest
(standard-library specializations 0.48 MB, outlined functions 0.46 MB, unnamed metadata and strings 0.40 MB)
belongs to no file. `ld -why_live` on one symbol of each family gives the chain that keeps it.

## What keeps optional code in a one-text binary

The `-why_live` chains show three reference patterns. Every Swift cut has to break all three for its family.

1. **Per-batch and per-node calls.**
   - `Presenter.apply(_:)` calls `MenuHost.sync()` and, through it, `ContextMenuHost.sync()`, along with
     `SwipeActionsHost.sync(changed:reshaped:)`.
   - `NodeView.applyStyle(_:)` reaches `RasterLoader.resized(_:)`.
   - `NodeView.renderFilter(fresh:)` reaches `BoxFilter.render`.
   - `NodeView.gradientDraws` reaches `Gradient.init`.
   - `NodeView.applyBoxLayer()` reaches `Presenter.segments`.
   - `NodeView.paragraphSpec()` reaches `TextEngine.atomIcons`, then `AtomIcons.svg`, then `exact_svg_image_scene`,
     which keeps the kernel's SVG scene and `exact_svg_file`.
   - `NodeView.embedPlatformView` reaches `WebViews.create`.
2. **Stored properties.** Swift reflection metadata keeps the type of every stored property of a live class.
   `ExactSession`'s field descriptor keeps `PlanURL` and its URL-session delegate, and `ExactApp` and `ExactLaunch`
   keep theirs.
3. **Launch and session wiring.**
   - `AppDelegate.application(_:didFinishLaunchingWithOptions:)` calls `WakeStore.install()`, which installs
     `TransferCarrier` and `RegionCarrier`.
   - `ExactSession.wire()` installs collection feedback, which keeps `exact_collection_feedback` and the runner's
     collections.
   - `ExactSession.expireBackgroundAction` keeps `exact_module_event` and with it the module seam.
   - `NotificationPresenter.shared` is made in `ExactSession.workEnded()`.
   - `ExactLaunch.ledger` calls `ExactSession.agent("outstanding")`, which keeps the whole production agent protocol
     (`exact_agent`, `runner::agent`).

On the Rust side the runner and the Apple host call capability code directly instead of through `RunnerLinks` or
`Links`. That is the regression below.

## The `url`, `idna`, `icu_normalizer` and `exact_grants` regression

LLP 1047.001's "host's I/O by use" took URL parsing out of an archive without `io`, by reading grant lines only
through `RunnerLinks.grants`. In the map they are back: `url` 20,536 B, `idna` 10,040 B, `icu_normalizer` and
`icu_collections` 7,898 B, `exact_grants` 15,996 B and `ibex2` 25,244 B, about 80 KB together. ibex2's Darwin
transport objects (`darwin_http.o`, `darwin_websocket.o`) are linked too, and their atoms are already inside those
families. Two chains bring them back, and the design review found more direct roots (below):

1. **`Bridge::emit_work` calls the grant grammar directly.** `exact_runner::grants::admitting` reaches
   `exact_grants::origin_of` and then `url::ParseOptions::parse`. The direct callers are:
   - `runner/src/runner/commit.rs:1052`, a stream request's grant line (`admitting_request`);
   - `runner/src/runner/faults.rs:408`, the agent's fault fixtures.

   Neither goes through `RunnerLinks.grants`.
2. **The production agent protocol is reachable.** `ExactSession.agent(_:)` is called from
   `ExactLaunch.ledger` (`Launch.swift:343`, `{"op":"outstanding"}`), `ExactTools`, `HeldState` and
   `HatchDiagnostics`. It reaches `exact_agent`, then `Host::answer_hold`, then `GrantSet::parse` in both
   `exact_grants` (through `save_file::covered`) and `ibex2::grant`.

**More roots.** The host parses grants directly:
- in `doc_watch.rs:120`, reached from `Host::finish` on every turn;
- in `store.rs:216`, `picker.rs:412` and `canvas_capture.rs:135`;
- in `Bridge::emit_work`'s surface admission.

**More production callers of `exact_agent`.** Beside the launch ledger, it is called from:
- `ExactTools` (call, poll and cancel);
- `NotifyActions` (ended background actions);
- the photo and document pickers;
- `WebModule` (frame requests);
- traces, `HeldState` and `HatchDiagnostics`.

The call paths in the runner and host are in the first public commit (a3c3170bc, 2026-11-01). The commit that added
them is in the private repository's history, which this checkout does not have.

The bytes leave only when the last grant read goes through the I/O link and the last production caller of
`exact_agent` has its own narrow export. That comes to about 0.12 MB with the part of the runner's agent module that only the dispatcher uses, and it is a
small design, not a pure regression fix: it spans the Rust runner and host, the C ABI, and six Swift families.

## The list

"Needed" is LLP 1047.001 D1's core: a plan with one `text` and one `button`.
- **N:** needed.
- **O:** not needed.
- **P:** in every build by a ruling, so it needs a new one.
- **D:** inspection; production keeps it only for a named use.

Confidence: high when the dead-strip falls exactly with the cut reference; medium when the family's direct bytes
come out but some helpers are shared with the core; low when the bytes are a judgment over code the dSYM attributes
loosely.

Risk counts the design work, from 1 (cut a reference) to 3 (a deep Swift seam). The table is ranked by bytes per unit
of risk within each tranche. The tranches are independent unless the table says otherwise.

| # | family | bytes, size build | N/O | what links it today | how it comes out | conf. | risk |
|---|---|---:|---|---|---|---|---|
| **A** | **Rust and C ABI reference cuts** | | | | | | |
| A1 | grants, URL parsing, the production agent protocol | ≈80,000 (`url` 20,536, `idna` 10,040, `icu_normalizer` and `icu_collections` 7,898, `exact_grants` 15,996, `ibex2` 25,244) + ≈40,000 of `runner::agent`'s 64,928 (its helpers that production uses stay) ≈ 120,000 | O | the two chains above, plus direct `GrantSet::parse` in `doc_watch.rs:120` (from `Host::finish`, every turn), `store.rs:216`, `picker.rs:412`, `canvas_capture.rs:135` and the surface grant in `Bridge::emit_work`; `exact_agent` from the launch ledger, `ExactTools`, `NotifyActions`, the pickers, `WebModule`, `Share`, Surfaces' `appRoots`, traces, `HeldState` and `HatchDiagnostics`; authentication's arming (`auth.rs`) also reaches `exact_grants` | every grant read through the I/O link; a narrow export per production agent operation, then `exact_agent` development only. Nothing is saved until the last root goes | medium | 2 |
| A2 | `Swappable` in an `off` entry | 14,936 (`exact_logic`, `exact_logic_abi`) | O | hello's generated entry wraps its data in `exact_logic::Swappable::off(...)`; `AppData::answer` calls `Swappable::call` and `activate_module`, also reached from `exact_module_event` | `contract::rust_entry` emits the plain source when the mode is `off` (D7); saves only after B1 removes the module-event root | low | 1 |
| A3 | the kernel's calc serializer | ≈2,400 (`write_numeric`; the parser and tokenizer stay) | O | `exact_kernel::style::calc::parse::env_parts` calls `cssparser::serializer::write_numeric` | write the number through `exact-num` | high | 1 |
| **B** | **the native module seam (LLP 1047.001 D11)** | | | | | | |
| B1 | the runner's native modules | ≈250,000 (315,408 of `runner::modules` less the provenance kept in the core) | O | `Extensions` is a concrete `Runner` field; about 150 functions there are called from the core runner; `exact_module_event` is reached from `ExactSession.expireBackgroundAction` | the amendment's module seam, made when the `modules` or `views` table is not empty; provenance moves to the core; `serde_json` stays (the core uses it) | medium | 3 |
| B2 | Swift: modules, wakes, notifications, activities, background transfer, the region carrier | 248,536 + 30,140 (`Regions.swift`) | O | `AppDelegate` installs `WakeStore`, which installs `TransferCarrier` and `RegionCarrier`; `ExactSession.workEnded` makes `NotificationPresenter.shared`; `NativeModule` is reached from the session | a capability target, `ExactModules`, linked when the module seam is (B1), with its launch hook registered by the generated composition | medium | 2 |
| **C** | **Swift capability targets (D4, as grouped lists and Markdown were done)** | | | | | | |
| C1 | lists, collections, reorder, swipe actions | 224,188 + 107,996 (`instance::collection`) | O | `Presenter.apply` calls `SwipeActionsHost.sync`; `ExactSession.wire` installs collection feedback; `Presenter.reset` calls `ReorderGroupHold.end` and then `CollectionHost.flush` | `ExactLists`, linked by `Capability::Collections` (`RunnerLinks.lists` already exists); the presenter calls registered batch hooks | high | 2 |
| C2 | SVG islands and filters, icons in text | 121,044 + 33,624 + 142,772 (kernel `svg`, `exact_svg_file`, `exact_apple::svg*`) | O | `NodeView.paragraphSpec` calls `TextEngine.atomIcons`, then `AtomIcons.svg`, then `exact_svg_image_scene` | `ExactSvg`, linked by `Capability::Svg` (the web already counts it); the text engine's atom-icon pass becomes a registered hook | high | 2 |
| C3 | images and rasters | 222,728 + 49,472 (`exact_raster`) | O | `NodeView.applyStyle` calls `RasterLoader.resized` | `ExactImages`, linked by a plan with an image node (QUEUE: "Apple: images linked by use") | high | 2 |
| C4 | menus and context menus | 234,848 (the dev menu is part of it) | O | `Presenter.apply` calls `MenuHost.sync()` every batch; `Presenter.press` calls `MenuHost.hideConfirmation` | `ExactMenus`, linked by a menu or context-menu node; the dev menu follows D6 (development) unless frame health's ruling keeps a production entry | medium | 2 |
| C5 | web, video, sound, MIDI, audio session, picture in picture | 131,763 + ≈36,000 (runner `sound`, `midi`; `presence`, `picker` and `surface_record` go with their own capabilities) | O | `ExactSession.afterFirstDraw` calls `NativeViews.prepareAppModule`, then `ElementHatches.releaseConfigurations`, then `VideoView.update`; `NodeView.embedPlatformView` calls `WebViews.create` | the arms' Swift halves become capability targets beside their loaded arms (the arms are already left out at store level 0) | medium | 2 |
| C6 | hatches and native views | 163,964 | O | `RasterBackend` calls `PackagePictures.ask`, then `NativeViews.openHatches`, then `HatchDiagnostics` | `ExactHatches`, linked when the plan names a native view | medium | 2 |
| C7 | box effects: gradients, shadows, filters, clips, backdrop | 122,528 + 16,176 (kernel `gradient`) | O | `NodeView.renderFilter`, `NodeView.gradientDraws` | linked by the style properties the plan uses, as `filter`, `clip-path` and `background-image` are on the web | medium | 2 |
| C8 | plan from a URL, update channels | 131,116 | O at store level 0 | `ExactSession` stores a `PlanURL`, kept by its reflection field descriptor | the store level, a composition input today (D2), excludes it, as it excludes `ExactUpdates` | high | 1–2 |
| C9 | content index, share, save, documents, clipboard, shortcuts, orientation | 123,376 | O | session and app wiring | one target per capability; the runner's share, documents and picker are already separate capabilities | medium | 2 |
| C10 | pointer, hover, drag, zoom, snapping, sticky, flights | 122,968 | O | `NodeView` and the scroll pump | `ExactDrag` exists; the rest join it or get their own target, by the properties the plan uses | low | 2 |
| C11 | tools (LLP 1111), Forma, live design, traces | 109,916 | D | `ExactTools` calls `session.agent`; `ExactSession.saveTrace`/`stopTrace` reach `ElementHatches` and `FrameSampler` | the tools' target by `plan.tools` (the amendment's tool seam); Forma and live design are development only (D6); traces by use of the trace API | medium | 2 |
| C12 | modals, sheets, dialogs, popovers | 115,968 | O | `NavigationHost.sync` calls `ModalHost.present`; `MenuHost.hideConfirmation` reaches `DialogSheets` | `ExactModals`, by dialog, sheet and popover nodes | medium | 2 |
| C13 | content regions (long documents) | 87,028 + 28,824 (`content_region_host`) | O | the presenter's region host | linked by the plan's region use | low | 2 |
| C14 | segments, radios, progress, switches | 108,236 | O (a plain button is not one) | `NodeView.applyBoxLayer` calls `Presenter.segments` | `ExactControls`, by those control kinds | medium | 2 |
| C15 | grid layout | 106,796 (`taffy::compute::grid`) | O | `LayoutTree::compute_mapped` dispatches every display mode through taffy's `compute_hidden_layout` | linked by `display: grid` in the plan; a kernel layout seam | medium | 2 |
| **D** | **deep Swift families** | | | | | | |
| D1 | navigation: stacks, bars, titles, tabs, covers, search | 507,484 | O (`Presenter.navigation` is `lazy`, so hello never makes one) | `NodeView.syncScroll` calls `Presenter.reveal`, then `NavigationHost`; `fitOwner`; tabs | a navigation target; the presenter and `NodeView` stop naming `NavigationHost` | low | 3 |
| D2 | fields, text areas, selection, keyboard, pickers, dates | 317,408 | O | `NodeView.fieldChanged` and the presenter's text-input paths; `PromptField` | a fields target; `NodeView`'s text-input delegate conformances move out (D4's "feature behaviour moves out of NodeView") | low | 3 |
| **R** | **rulings, not designs** | | | | | | |
| R1 | frame health (card, overlay, sampler, report) | 300,936 | P (LLP 1079.000: every build) | the four-finger dev menu, `ExactSession.stopTrace` | a ruling that production links it only on request, or loads it | high | ruling |
| R2 | the Rust panic machinery (backtrace symbolization) | 106,876 (`gimli`, `addr2line`, `rustc_demangle`, `object`, `std::backtrace_rs`) | O | std's panic handler prints a backtrace, even with `panic=abort` | nightly `build-std` with `panic_immediate_abort`; D9 declined nightly `build-std` | high | ruling |
| R3 | Rust unwind tables | 0 | O | measured 2026-11-18: `-C force-unwind-tables=no` reaches rustc and changes no row or section by a byte under fat LTO; what remains is the prebuilt standard library's | only with R2's `build-std` | high | ruling |

## Totals

Direct bytes only. The design review held, rightly, that the overhead credit (a share of the 1.34 MB of unattributed
Swift, of the unwind tables and of the C ABI) is a hypothesis. It is not claimed here: each tranche's link map
measures it.

The amendment's §9.3 orders the work differently from the tranches above: menus and modals join navigation, grid
joins the runner lane, and the icon pass stays in the core. Its ordering gives these rows:

| after | direct | size row |
|---|---:|---:|
| lanes R and A, and family lanes F1 to F3 (HatchDiagnostics' ≈43 KB counted once, in A1) | 2.80 MB | 9.69 MB |
| navigation with modals, menus, fields | 1.18 MB | 8.52 MB |
| frame health and the panic machinery (two rulings) | 0.41 MB | 8.11 MB |

**Does it reach 8,000,000 B?** With the two rulings, on paper; without them, only if the overhead comes out.
- **By direct bytes**, with what M1 attributed (Progress, below), every step reaches about 8.40 MB, and the two
  rulings take it to about 7.84 MB.
- **The unmeasured overhead** could add 0.3 to 0.4 MB. That covers the standard-library specializations, outlined
  functions and Swift metadata serving only removed families. Each tranche's link map measures it.
- **The table above** is the list's first totals, before M1; the unwind-table lever it once counted saves nothing.

**What the next tranche past the families costs in design.**
- **Navigation, modals, menus and fields** need `NodeView` to keep no delegate conformance or `@objc` action for a
  feature. The presenter's scroll, reveal and press paths must call typed interfaces instead of `NavigationHost`,
  `MenuHost`, `DialogSheets` and `PromptField`.
- **They interact with each other**, so they are one lane, in order.

## Smaller items found on the way

- **`ExactLaunchParts`' 42 KB and `dtoa`'s 108 KB were artifacts of symbol gaps.** The map has 18 B and 312 B.
- **Case-mapping tables are about 11 KB, not 158 KB.** Two callers keep them: the kernel's `text-transform`
  (`case::shown`, which "maps directly" off the web) and the runner's `toUpperCase` (`case::uppercase_bounded`).
  Linking them by use is worth doing only alongside another kernel seam.
- **The unnamed Rust constants are 459,453 B** (`__const` atoms with no name in the archive), and the map gives them no
  owner. They include the baked plan, generated tables and string data. Attributing them takes a per-crate pass with
  the LTO object's relocations, which tie each constant to the functions that read it. LLP 1047.001 §9.3's M1 is that measurement.

## Progress

The design is LLP 1047.001 §9 (D10: every Swift family is a capability target the composition selects; D11: the
runner's seams; §9.3: the order of work). Landed, size row:

| step | commit | size row after |
|---|---|---:|
| the tripwire and its 100 KB steps | 27e13aca0 | 12,489,072 B |
| R1: call provenance into the runner core (a move) | 24de53c85 | unchanged |
| R4a: child processes and the work clock out of the modules (a move) | 6429d33fc | unchanged |
| R3: grid layout linked by use (cut list C15) | bfc7f585a | 12,389,184 B |
| R4b to R4e: wakes into the core; tool calls and presence secrets behind their seams (moves) | e50d50b66 | unchanged |
| C0 S1, S2a: the core reaches SVG and CSS filter only through link slots (moves) | 0b09b06a4 | +96 B |
| A1: hatch diagnostics out of the production composition | 2c30ff62e | 12,340,048 B |
| A3: the calc serializer (cut list A3; under a 16 KB page on this row) | 5269a0603 | unchanged |
| R4f: native modules behind their seam (a move) | 734742f72 | unchanged |
| the kernel's metrics share one value pool | 863eb0dbb | 12,307,392 B |
| device lines checked without the device table's build facts | d00405a8d | 12,290,976 B |
| R4g: Apple links native modules, tool calls and presence secrets by use (cut list B1) | c80a452b8 | 11,825,920 B |
| R2: every grant read through the I/O link | 56f303560 | −16,608 B |
| A2: each production agent operation its own export; the dispatcher development's (cut list A1) | 055dfb43b | −181,168 B |
| C0 S1 to S4d: the core reaches SVG, CSS filter, images, lists, hatches, optional controls, flights and sticky only through link slots | 262d67874 | about +70,000 B, which leaves with each family |
| F1: CSS filter its own product | ce50a49b2 | −33,264 B |
| F1: SVG its own product, the filter engine with it (cut list C2) | 0e7168690 | −318,128 B, to 11,327,456 B |
| F3: images their own product, the raster crate with them (cut list C3) | 684c8ad80 | −371,664 B |
| F2: lists, swipe actions and row cells their own product (cut list C1's Swift) | bcebe6eba | −266,816 B, to 10,721,312 B |
| the motion handover to Core Animation linked only where something can move | 89f89da69 | −49,728 B |
| the runner's font-axis motion a seam | b68ec63e3 | −33,168 B |
| F4: the native module and element hatches their own product (cut list C6) | 3878daeb4 | −248,800 B |
| C0 S5a, S5b: the core reaches media and the module seam's session families only through links | dd4a54a91 | about +33,000 B, to 10,439,232 B |
| an archive without I/O loads none of ibex2's HTTP or WebSocket objects, which had rooted its stores | 09bc0e150 | −268,720 B |
| child-process admission and widget publication through the I/O link | 902cda43c | under a page |
| C0 S5c, S5d: the core reaches pickers, sharing, traces, the wake lock, the plan connection and Forma only through links | 1ec75c6eb | about +16,500 B |
| the animation, filter and clip-path grammars linked by use on native hosts | b9a4447a2 | −32,992 B |
| F1: optional form controls their own product (cut list C14) | eac184563 | −135,760 B |
| F1: shared-element flights and `position: sticky` their own products | 4d7e47995 | −49,520 B, to 10,202,080 B |
| N0a: the core reaches iOS navigation and route modals only through a link | 8633bd19d | +16,736 B |
| the runner's list engine a seam, linked with lists (cut list C1's Rust) | 6c8548e48 | −133,040 B, to 10,085,776 B |
| a grid row selects grid on Apple, and grid's grammars link with the algorithm | 261bc7853 | −16,544 B |
| N0b: the iOS core and menus reach the `dialog` sheet only through a link | cee801a97 | +64 B |
| B2: notifications, the host-event mailbox, Live Activities and the wake carriers their own products | 89c9afaa9 | −258,848 B |
| C11: no plan connection in production, and Forma linked only with native views | d92c68ee8 | −169,616 B, to 9,722,832 B |
| CSS gradients their own product | ae3c7134d | −34,432 B |
| the transition and gradient grammars and the exit check linked by use on native hosts | eb4e8b941 | −16,688 B |
| exits, layout and height transitions and list moves behind the turn's motion link | 2b53c66e0 | −16,672 B |
| C9: the pickers their own product | 83f6a7e1e | −54,768 B |
| C11: traces their own product | 0d103a3bf | −82,928 B |
| C9: the wake lock its own product | 36d7baf41 | under a page, to 9,550,448 B |
| C13: content regions linked only by an entry that registers one, with their Rust behind a seam | a4c46784a | −215,744 B |
| no build channel, development plan or development link in production | 90bf0223f | −69,056 B |
| C8: video, web views, sound and MIDI their own products | 44df8d345 | −201,232 B |
| the runner's voice table, transport and MIDI queue a seam | 1db527b8e | −66,320 B |
| what Core Animation plays for a box its own product | 10dea87d0 | −16,784 B |
| N1: navigation with route modals its own product (cut list C2) | e27820042 | −635,152 B, to 8,345,408 B |
| the lists' reorder and snapshot decoding the lists family's | d84905dd5 | −66,096 B |
| the Rust host's drags and the gradient serializer linked by use | 78a758b23 | −33,200 B |
| the heavy leaves' hold a target of video, web views and native views | bba0b3a7d | −33,040 B |
| transform and height drag handles the drag product's | 16a8c2c6e | −49,648 B |
| iOS text selection its own product | 330f7d957 | −67,024 B |
| widget publication its own product, the headless session the snapshot module's | 756a622eb | −117,584 B |
| Canvas 2D's host turn, captures and exports linked by use | 06984a58d | −33,152 B |
| C11: the tool door its own product | b23af37ec | −16,544 B |
| C9: the content index its own product | bebd075f9 | −71,808 B, to 7,904,944 B |
| icons in text the Markdown product's | f607bac14 | −17,504 B, to 7,871,008 B |

A second link map, at 51002ee77, measured what the first declined to claim: the overhead (standard-library
specializations, outlined functions, metadata, unwind sections) leaves with the families at 0.17 B for each direct
byte, 0.33 for a Swift byte. Re-priced with that ratio, the steps still ahead (the lists' Rust engine, optional
controls, flights, media, wakes and notifications, tools and traces, the plan from a URL, share, navigation with
route modals, dialogs, menus, fields, content regions, and about 105 KB of Rust the first list missed) end near
8.2 MB. Frame health is about 0.5 MB of the row with its overhead and the panic machinery about 0.24 MB; either
ruling closes the gap.

A third link map, at d92c68ee8 (9,722,832 B), classes every byte by the type that reaches it rather than by file,
which is what the second map's three large misses needed (controls, the session families and the plan connection
each left with bytes it had counted as core). At the ratios measured on it (a Swift byte leaves with 0.42 B of
overhead, a Rust byte with 0.18 B) the steps still ahead end near 7.86 MB with neither ruling: pickers and traces
to 9.57 MB, media to 9.35 MB, navigation with dialogs, menus and fields to 8.16 MB, tools, the small capability
families and share to 8.04 MB, content regions to 7.86 MB. If the deep families shed less overhead than the
measured ratio, that is nearer 8.04 MB. The map also found about 0.56 MB that no step owned and no plan of
hello's reaches, each linked by use rather than shaved: what only a development-shaped caller keeps (the agent's `state` reader, the build channel's status report, the
dev menu's reload, the development link; about 113 KB), box effects and the gradient grammar (about 99 KB),
pointer, drag, zoom and snapping (about 84 KB), widget publication in an app with no entries (about 75 KB), text
selection (about 56 KB), the transition and gradient grammars with the height, exit and layout transitions
(about 50 KB; the rest of the motion engine shows a still plan's transforms, opacity and colour and stays), and
smaller remnants. With the five largest the list ends between 7.53 and 7.71 MB.
Frame health is about 0.43 MB and the panic machinery about 0.24 MB; neither ruling is needed for the target.

A fourth link map, at 4e67058fd (8,345,408 B, with navigation, media and content regions out), prices what is
left. The third map's ratios were too high (a direct byte leaves with about 0.22 B of overhead, not 0.42); its
steps came in above their prices because it filed direct bytes under the wrong owner (closures and
specializations under the first type they name, methods on `Host` and `Runner` as core). What was built at that
point and not yet landed (the lists remnants, both halves of drag, tools, the content index, widgets out of the
core) ends at 7.99 MB, and text selection at 7.92 MB; the iOS dialog sheet, menus, fields, icons in text, the
heavy leaves' hold and share end near 7.28 MB, or 7.44 MB if each family takes only its own types. Below that, what
no step owned is about 0.22 MB, each linked by use: Canvas 2D and canvas capture (about 45 KB), arrangement
(about 40 KB), shadows, masks, materials and backdrop (about 37 KB), bars in edge slots and route-change marks
(about 26 KB), glass groups (about 23 KB), snapping and zoom (about 22 KB). Past those, what is left in the node
view, presenter, session and runtime is members of a few KB each, and cutting there would be shaving: the rest of
a half megabyte below the list's end needs one of the two rulings (frame health, about 0.43 MB on the row with
the dev menu and the agent's `state` reader that only it keeps; the panic machinery, about 0.24 MB).

R4g took 481,888 B off the size row against the list's estimate of about 250,000 B (629,808 B off balanced and
1,265,968 B off speed): `runner::modules` went from 370,556 B of code to 8,104 B, the drop code of its state,
which 40b2d2c8c then put behind the seam. The core kept about 33 KB of what was in `modules/`: provenance,
wakes and background actions, child processes and the unlinked seam.

The rows after R3 are each lane's before-and-after on one Mac at its own base; main also grew about 33 KB from
other work between bfc7f585a and 0b09b06a4. M1 attributed the Rust archive's 459 KB of unnamed constants: `url`'s
host parsing reads 100,686 B of them (so cut-list row A1 is worth about 0.22 MB), panic location records and
their file paths are 113,253 B (the panic-machinery ruling's), the kernel's `METRICS` table is 41,664 B, and the
runner's `DEVICES` table about 20 KB. By direct bytes the list then reaches about 8.40 MB without the two
rulings and about 7.84 MB with them.

Two cuts on the list are rulings, not designs, and are not being built: frame health out of the production core
(LLP 1079.000) and the panic machinery (nightly `build-std`, which LLP 1047.001 D9 declined).

The size row is at 7,871,008 B and its ceiling at 7,970,000 B, both under the 8,000,000 B target. What is built
or traced below that and not landed, and what the moves still owe, are in QUEUE.md.

## Checked after closing

Main at 83d7bcfd3, 2026-11-19.

**Real input.** Production builds for the iOS simulator, each linked by its own plan, were driven with real HID
events: a simulator (iPhone 18 Pro, iOS 27.0) inside a macOS guest whose SSH process may post events, so a click
in the guest is a touch in the simulator.

| app | what a touch did |
|---|---|
| hello | launches and draws |
| Caltrain | a button opens the station list; a tapped field takes hardware keys; a tap pushes the web deck (an `iframe`); a left-edge swipe goes back |
| Shelf | a segment filters; a row pushes its detail; an edge swipe pops it; a row swipe reveals Delete; `+` presents the route sheet, where typed text, a stepper and Save add the book to the list |
| Markdown | a long press selects a word, with handles and Copy; Open… presents the document picker |
| interaction gallery | a tapped photo opens its viewer; a held drag moves it and release sends it back; a card dragged by its grip to the list's edge scrolls the list and drops in a new place ("Placed photo-00000 at position 3") |
| orientation fixture | Dialog presents the `dialog` sheet; Close leaves the page pixel-identical |
| UI gallery | a grouped row pushes; a segment switches; the bar's back button pops; a long press lifts a tag with its menu |

A long press on a link in the Markdown reader selects its word rather than offering the link's menu; the build at
bba0b3a7d, before text selection moved, does the same. Caltrain's widget is listed in the simulator's widget
gallery, draws its preview there, and on the home screen shows the app's published times. On the Mac (macOS 27.2
in a guest), production Markdown Stress in its windowed mode takes a real drag as a selection across lines and a
drag of the scroller's knob as a scroll that mounts the blocks it reaches.

**The suites.** The gate passes (29.8 s). Production iOS, the all-linked development iOS build, tvOS and production
macOS build. The reload tests, the runner (963), the Apple host (608), the kernel (662) and the production link
test pass. The whole Mac and iOS XCTest suites ran, and every failure was rerun and compared with 02639b8ea,
where the cuts began: none is one of these moves', and QUEUE.md names each with the commit it starts at.

**The list engine's seam.** `bun scripts/metrics.mjs --scaling` at 3fd3821cb, before the seam, and at main, three
runs each on an idle x86-64 Linux box: reorder, bump and topology at 300, 3,000 and 10,000 rows are within 3 % in
runner update and layout time, with batches byte-identical. `--list-memory --collections`, eighteen repeats each:
the median ratio over its phases is 1.006, and twenty full traversals of 1,000 and of 25,000 rows are 1.02.
