// build.ts
import('esbuild').then((esbuild) => {
  import('esbuild-plugin-wasm').then((wasmLoader) => {
    // "build-base": "esbuild ./src/main.ts --bundle --outfile=out/main.js --external:vscode --format=cjs --platform=node --target=node16",
    esbuild.build({
      entryPoints: ['src/main.ts'],
      outfile: 'out/main.js',
      bundle: true,
      platform: 'node',
      target: 'node16',
      format: 'esm',
      external: ['vscode'],
      plugins: [
        wasmLoader.wasmLoader({
          mode: 'embedded'
        })
      ]
    });
  });
});



