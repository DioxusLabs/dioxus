// One-shot setup for a `HydrationChannel` flush, called from Rust right
// before draining the queued sledgehammer ops. Binds the channel to the live
// mutation `BaseInterpreter` (so hydration ops can write into `base.nodes` /
// install listeners on shared `base.global`/`base.local`) and seeds the
// per-pass walker state.
//
// ElementIds are now passed inline as op args (no parallel `hyIds` array).
// `under` is the root-node array `EnterRoot(i)` indexes into.
// `frames` + `frameWrap` form a parallel-array frame stack: `frameWrap[i]`
// is 1 for transparent-wrapper auto-descents, 0 for user-emitted
// `BeginChildren` frames. `EndChildren` drains wrappers above the user
// frame before popping the user frame itself.

export function installHydrationState(
  channel: any,
  base: any,
  under: Node[]
): void {
  channel.base = base;
  channel.hyUnder = under;
  channel.cursor = null;
  channel.frames = [];
  channel.frameWrap = [];
  channel.lastMapped = null;
  channel.lastMappedId = 0;
  channel.chainTail = null;
  channel.currentRootParent = null;
}

// Push a virtual anchor onto the mutation stack for an empty streaming-suspense
// chunk. The mutation interpreter records the loading slot position through
// this anchor during `replace_with(loading_id, 1)`.
export function pushHydrationVirtualRoot(base: any): any {
  const sentinel = base.createVirtualAnchor(null, null);
  base.stack.push(sentinel);
  return sentinel;
}

// After suspense resolution, bind the resolved scope's placeholder ElementId
// to the same virtual anchor. By then `replace_with` has populated the anchor's
// parent/before fields through `applyChunk`.
export function claimHydrationVirtualRoot(
  base: any,
  id: number,
  sentinel: any
): void {
  base.nodes[id] = sentinel;
}
