# A rolling stat that pauses for accessibility

Gate the timer itself so the stat stays still while a screen reader may be on,
at large text sizes, under reduced motion, or under the person's gentler-motion
setting. A manual Next button keeps every stat available.

```contract
shape Viewport
  screenReader: string
  accessibilityTextSize: bool
  prefersReducedMotion: bool

component App
  resource viewport = exactViewport() as shape Viewport
  state stat = 0
  state gentlerMotion = false
  derive rotates = viewport.screenReader == "off" and not viewport.accessibilityTextSize and not viewport.prefersReducedMotion and not gentlerMotion
  task rotate when rotates
    every(3000, next)
  action next
    stat = (stat + 1) % 3
  action toggleMotion
    gentlerMotion = not gentlerMotion
  view
    column
      text (stat == 0 ? "12 galleries" : (stat == 1 ? "48 artists" : "240 works")) testId="stat" font-variant-numeric="tabular-nums"
      button "Next stat" press=next testId="next"
      button "Gentler motion" aria-pressed=gentlerMotion press=toggleMotion testId="gentler"
```

`"unknown"` is possibly on, so this optional rotation requires `"off"`. Browsers
cannot detect screen readers and keep the manual advance. Android's large-text
range starts at the system font scale of 1.3; UIKit uses its accessibility text
categories. Neither condition depends on the font size the app authors.
