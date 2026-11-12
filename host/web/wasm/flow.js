import { host, loadAfterPaint, views, agentMode, log, now, followOffset, readOut } from "../glue.js";
import { send, applyBatch } from "./commit.js";
export function flowBatch(batch) {
  const op = batch.ops?.find((op) => op.op === "textflow");
  if (op) {
    host.flowContexts = op.contexts;
  }
  host.flowDue = batch.timer_due_ms ?? null;
  host.flowLanding = !!batch.landing;
  host.flowFrames = !!batch.frames;
  host.flowWaits = batch.waits ?? [];
  host.flowTolerance = batch.timer_tolerance_ms ?? 0;
  host.ticker?.update(host.flowDue, host.flowFrames, host.flowWaits, host.flowTolerance);
  if (host.textflow) {
    host.textflow.afterBatch(batch);
    return;
  }
  if (!host.flowContexts.length || host.flowLoading) {
    return;
  }
  host.ticker?.dispose();
  host.ticker = null;
  const generation = host.incarnation;
  host.flowLoading = loadAfterPaint("./textflow-glue.js", "createTextFlow").then(async (create) => {
    if (generation !== host.incarnation) {
      return;
    }
    const controller = await create({
      views,
      agentMode,
      log,
      now,
      advance: () => {
        followOffset(now());
        send(host.wasm.exact_advance(now(), 0));
      },
      present: host.present,
      admitTimers: (kinds, cutoff) => {
        const batch = JSON.parse(readOut(host.wasm.exact_advance(cutoff, 16 | kinds << 8)));
        applyBatch(batch);
        return batch;
      }
    });
    if (generation !== host.incarnation) {
      controller.dispose();
      return;
    }
    host.textflow = controller;
    host.textflow.afterBatch({
      ops: [{
        op: "textflow",
        contexts: host.flowContexts
      }],
      timer_due_ms: host.flowDue,
      frames: host.flowFrames,
      waits: host.flowWaits,
      timer_tolerance_ms: host.flowTolerance
    });
  });
  host.flowLoading.catch((error) => {
    if (generation === host.incarnation) {
      log(`textflow module: ${error}`);
    }
  });
}
