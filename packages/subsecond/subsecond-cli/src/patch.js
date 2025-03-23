/**
 * Patches a WebAssembly module by replacing its exports with those from another module.
 *
 * @param {WebAssembly.Exports} base - The base WebAssembly exports object to be patched.
 * @param {WebAssembly.Exports} patch - The WebAssembly exports object containing the patch functions.
 * @throws {TypeError} If the export names cannot be parsed as integers.
 */
function patchWasm(base, patch) {
  const patchExports = Object.entries(patch);

  // extract the export names from the patch table
  const patchVec = new Uint32Array(patchExports.length * 2);

  // iterate through the patch exports and get the key and value
  let idx = 0;
  for (const [key, value] of patchExports) {
    patchVec[idx] = parseInt(base[key].name);
    patchVec[idx + 1] = parseInt(value.name);
    idx += 2;
  }

  // call the patch function
  base["__subsecondPatch"](patchVec);
}

/**
 * Loads a WebAssembly module from a given URL, instantiates it, and applies a patch
 * to the provided base object using the module's exported functions.
 *
 * @param {WebAssembly.Exports} base - The base object to be patched with the WebAssembly module's exports.
 * @param {string} url - The URL of the WebAssembly module to fetch and instantiate.
 * @returns {void} This function does not return a value.
 */
function loadAndPatch(base, url) {
  fetch(url)
    .then((response) => response.arrayBuffer())
    .then((bytes) => WebAssembly.instantiate(bytes))
    .then((result) => {
      patchWasm(base, result.instance.exports);
    });
}
