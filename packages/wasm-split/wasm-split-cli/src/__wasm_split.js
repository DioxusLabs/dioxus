// when running the harness we need to make sure to uncommon this out...

export function makeLoad(url, deps, fusedImports, initIt) {
  let alreadyLoaded = false;
  let loadingPromise = null;

  const notifyCallback = (callbackIndex, callbackData, success) => {
    if (callbackIndex !== undefined) {
      const initSync = initIt || globalThis.__wasm_split_main_initSync;
      const mainExports = initSync(undefined, undefined);
      mainExports.__indirect_function_table.get(callbackIndex)(
        callbackData,
        success
      );
    }
  };

  return async (callbackIndex, callbackData) => {
    if (!alreadyLoaded) {
      if (!loadingPromise) {
        loadingPromise = (async () => {
          try {
            const depResults = await Promise.all(deps.map((dep) => dep()));
            if (depResults.some((loaded) => loaded === false)) {
              throw new Error(
                `Failed to load dependencies for wasm-split module: ${url}`
              );
            }

            const response = await fetch(url);
            if (response.ok === false) {
              throw new Error(
                `Failed to fetch wasm-split module ${url}: ${response.status} ${response.statusText}`
              );
            }

            const initSync = initIt || globalThis.__wasm_split_main_initSync;
            const mainExports = initSync(undefined, undefined);

            let imports = {
              env: {
                memory: mainExports.memory,
              },
              __wasm_split: {
                __indirect_function_table:
                  mainExports.__indirect_function_table,
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

            let new_exports = await WebAssembly.instantiateStreaming(
              response,
              imports
            );

            for (let name in new_exports.instance.exports) {
              fusedImports[name] = new_exports.instance.exports[name];
            }

            alreadyLoaded = true;
          } finally {
            if (!alreadyLoaded) {
              loadingPromise = null;
            }
          }
        })();
      }

      try {
        await loadingPromise;
      } catch (e) {
        console.error(
          "Failed to load wasm-split module",
          e,
          url,
          deps,
          fusedImports
        );
        try {
          notifyCallback(callbackIndex, callbackData, false);
        } catch (callbackError) {
          console.error(
            "Failed to notify wasm-split callback",
            callbackError,
            url
          );
        }
        return false;
      }
    }

    notifyCallback(callbackIndex, callbackData, true);
    return true;
  };
}

let fusedImports = {};
