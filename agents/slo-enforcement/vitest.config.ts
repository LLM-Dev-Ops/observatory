import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.ts', 'contracts/**/*.ts'],
      exclude: ['tests/**', 'dist/**'],
    },
    testTimeout: 10000,
  },
  resolve: {
    alias: {
      '@contracts': './contracts',
      '@src': './src',
    },
  },
});
