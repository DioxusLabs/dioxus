// when running the harness we need to make sure to uncommon this out...

export function makeLoad(url, deps, fusedImports, initIt) {
  let loadPromise;
  return async (callbackIndex, callbackData) => {
    await Promise.all(deps.map((dep) => dep()));
    if (!loadPromise) {
      loadPromise = (async () => {
        try {
          const response = await fetch(url);
          const initSync = initIt || globalThis.__wasm_split_main_initSync;
          const mainExports = initSync(undefined, undefined);

          let imports = {
            env: {
              memory: mainExports.memory,
            },
            __wasm_split: {
              __indirect_function_table: mainExports.__indirect_function_table,
              __stack_pointer: mainExports.__stack_pointer,
              __tls_base: mainExports.__tls_base,
              memory: mainExports.memory,
            },
          };

          for (let mainExport in mainExports) {
            imports["__wasm_split"][mainExport] = mainExports[mainExport];
          }

          for (let name in fusedImports) {
            imports["__wasm_split"][name] = fusedImports[name];
          }

          const new_exports = await WebAssembly.instantiateStreaming(response, imports);
          for (let name in new_exports.instance.exports) {
            fusedImports[name] = new_exports.instance.exports[name];
          }
        } catch (e) {
          loadPromise = undefined;
          console.error(
            "Failed to load wasm-split module",
            e,
            url,
            deps,
            fusedImports
          );
          throw e;
        }
      })();
    }
    await loadPromise;

    if (callbackIndex !== undefined) {
      const mainExports = (initIt || globalThis.__wasm_split_main_initSync)(undefined, undefined);
      mainExports.__indirect_function_table.get(callbackIndex)(callbackData, true);
    }
  };
}

let fusedImports = {};
