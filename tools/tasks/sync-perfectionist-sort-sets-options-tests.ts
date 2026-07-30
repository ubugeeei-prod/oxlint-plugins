// Captures the v5.10.0 sort-sets authored suite through the shared
// Perfectionist fixture synchronizer without moving the repository-wide v5.9.1
// submodule pin.

process.argv.push('--sort-sets');
const synchronizer = new URL('./sync-perfectionist-sort-imports-options-tests.ts', import.meta.url);
await import(synchronizer.href);
