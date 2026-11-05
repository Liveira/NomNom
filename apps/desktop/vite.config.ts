import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', sourcemap: false, chunkSizeWarningLimit: 300 },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
});
