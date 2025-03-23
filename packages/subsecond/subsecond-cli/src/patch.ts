function loadAndPatch(
  base: WebAssembly.Exports,
  url: string,
) {
  fetch(url)
    .then((response) => response.arrayBuffer())
    .then((bytes) => WebAssembly.instantiate(bytes))
    .then((result) => {
      patchWasm(base, result.instance.exports);
    });
}

function patchWasm(
  base: WebAssembly.Exports,
  patch: WebAssembly.Exports
) {
  const patchExports = Object.entries(patch);

  // extract the export names from the patch table
  const patchVec = new Uint32Array(patchExports.length * 2);

  // iterate through the patch exports and get the key and value
  let idx = 0;
  for (const [key, value] of patchExports) {
    patchVec[idx] = parseInt((base[key] as Function).name);
    patchVec[idx + 1] = parseInt((value as Function).name);
    idx += 2;
  }

  // call the patch function
  (base["__subsecondPatch"] as Function)(patchVec);
}
