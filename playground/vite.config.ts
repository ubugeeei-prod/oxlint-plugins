import { defineConfig } from 'vite-plus';

export default defineConfig({
  // The playground is a static client-side app; everything runs in the browser.
  base: './',
  build: {
    target: 'es2022',
    outDir: 'dist',
    sourcemap: true,
  },
});
