// Captures the v5.10.0 sort-array-includes authored suite through the shared
// Perfectionist fixture synchronizer without moving the repository-wide v5.9.1
// submodule pin.

process.argv.push('--sort-array-includes');
const synchronizer = new URL('./sync-perfectionist-sort-imports-options-tests.ts', import.meta.url);
await import(synchronizer.href);
