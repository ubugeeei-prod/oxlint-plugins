import { defineConfig } from 'vite';

export default defineConfig(async () => {
  const { voidPlugin } = await import('void');

  return {
    plugins: [voidPlugin()],
    build: {
      outDir: 'dist',
      sourcemap: true,
    },
  };
});
