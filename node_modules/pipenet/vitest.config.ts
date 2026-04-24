import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    exclude: ['node_modules', 'dist', 'server'],
    include: ['src/**/*.spec.ts'],
    testTimeout: 30000,
  },
});

