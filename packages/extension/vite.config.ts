import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';

export default defineConfig({
  plugins: [
    viteStaticCopy({
      targets: [
        { src: 'static/icon.png', dest: '.', rename: { stripBase: true } },
        { src: 'pkg/dioxus_ext_bg.wasm', dest: '.', rename: '../main.wasm' },
      ],
    }),
  ],
  build: {
    lib: {
      entry: 'src/main.ts',
      formats: ['cjs'],
      fileName: () => 'main.js',
    },
    rolldownOptions: { external: ['vscode'] },
    outDir: 'dist',
  },
});
