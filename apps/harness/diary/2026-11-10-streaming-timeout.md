surfaces: terminal
agent: Codex
task: correct streaming response timeouts

### Rough

ureq's response-header timeout also capped the streaming body. A loopback
reply that kept sending data failed halfway through a four-second test with
a two-second limit. The transport now bounds each wait for input instead.

The manual terminal fixture initially omitted input arming and sent no
prompt. Using the existing terminal CLI over an offline data source drove
the screen without background catalog requests. A 100-column drive clipped
the fixture's unbroken token string, though its completion marker was visible.

A partial line could keep the reader alive after the caller's idle error.
Each request now stops its reader on exit without cancelling the turn.
The original regression's timed producer also depended on scheduling;
the tests now advance a controlled clock and answer scripted reads.

### Lean in

The shared provider transport lets one fix cover every HTTP model provider.

### Needed

none

### Checkpoints

- Provider tests: continuous replies finish across four minutes of controlled
  time. Silence closes the connection before or after the response headers;
  idle errors and callback completion stop a partial-line reader. A shorter
  ureq deadline retains its timeout reason.
- Terminal drive: rough setup as above, then a 180-column drive completed in
  137 seconds with all 135 content tokens and the final marker visible.
