surfaces: android
agent: Claude Code (Claude Opus 5.5)
task: Draw a radio group as Material's segmented buttons, and add the oracle's specimen for them.

### Rough

The specimen (`specimens/segmented-buttons.contract`) and its Compose side (`reference/`, `SingleChoiceSegmentedButtonRow`) were added and never run: the Android emulator does not boot on the Mac this was written on, and there was no phone. The specimen door's picture of the group was not compared with Compose's, so the oracle has not yet checked the `segmented-button-` rows against the library. The door now writes each segment's painted label as a text box (`linux/src/main.rs`), since a segment's label is painted, not a node; that path has not been run either.

The numbers were read from the pinned Compose Material 3 1.4 jar in the Gradle cache instead (`OutlinedSegmentedButtonTokens`, `SegmentedButtonDefaults`, `SegmentedButton.kt`'s `IconSpacing`), by reading the constants out of the class files: there is no JDK on the Mac, so `javap` could not run.

What was seen came from the Linux presenter in Material's look (its tests in `host/linux/src/presenter/material_tests.rs`), which draws what the reader shows.

### Lean in

The presenter's Material look runs headless in a test process of its own, so the whole segmented row, its states and its right-to-left form could be looked at and asserted without a device.

### Needed

none

### Checkpoints

- specimen door build: smooth.
- oracle run against Compose: not run (no device or emulator).
