import { existsSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const lockRoot = resolve(root, 'target', '.napi-setup-locks');
// Per-package locks prevent multiple Vitest workers from racing on the same
// `napi build` invocation. Concurrent `napi build` invocations on the same
// package corrupt napi-rs intermediate type files (ENOENT on the
// intermediate dts file).
const BUILD_TIMEOUT_MS = 10 * 60 * 1000; // 10 minutes covers cold cargo builds
const STALE_LOCK_MS = 15 * 60 * 1000;
const POLL_INTERVAL_MS = 250;

// Packages whose Vitest suites depend on NAPI bindings. Normally `vp build`
// produces these before `vp test`; this is the fallback for direct test runs.
const nativePackages = [
  {
    name: '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
    binding: 'npm/no-forbidden-identifiers/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-comments',
    binding: 'npm/eslint-comments/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-security',
    binding: 'npm/security/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-cypress',
    binding: 'npm/cypress/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-e18e',
    binding: 'npm/e18e/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-stylistic',
    binding: 'npm/stylistic/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-react-refresh',
    binding: 'npm/react-refresh/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-regexp',
    binding: 'npm/regexp/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-mocha',
    binding: 'npm/mocha/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-simple-import-sort',
    binding: 'npm/simple-import-sort/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-unused-imports',
    binding: 'npm/unused-imports/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-storybook',
    binding: 'npm/storybook/native.js',
  },
];

function lockPathFor(pkg) {
  const safe = pkg.name.replace(/[@/]/g, '_');
  return resolve(lockRoot, `${safe}.lock`);
}

function tryAcquireLock(lockPath) {
  try {
    mkdirSync(lockPath);
    return true;
  } catch (err) {
    if (err.code === 'EEXIST') return false;
    throw err;
  }
}

function isStaleLock(lockPath) {
  try {
    const stat = statSync(lockPath);
    return Date.now() - stat.mtimeMs > STALE_LOCK_MS;
  } catch {
    return false;
  }
}

const sleepBuffer = new Int32Array(new SharedArrayBuffer(4));

function sleepSync(ms) {
  // True blocking sleep without busy-waiting. The Atomics.wait timeout is
  // exactly what we want here while we poll for the lock holder to finish.
  Atomics.wait(sleepBuffer, 0, 0, ms);
}

function waitForBinding(pkg, lockPath) {
  const bindingPath = resolve(root, pkg.binding);
  const deadline = Date.now() + BUILD_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (existsSync(bindingPath)) return true;
    if (isStaleLock(lockPath)) {
      try {
        rmSync(lockPath, { recursive: true, force: true });
      } catch {
        // another worker may have removed it concurrently
      }
      return false;
    }
    sleepSync(POLL_INTERVAL_MS);
  }
  return false;
}

function buildPackage(pkg) {
  const result = spawnSync('pnpm', ['--filter', pkg.name, 'build'], {
    cwd: root,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    throw new Error(`Failed to build NAPI bindings required by Vitest for ${pkg.name}.`);
  }
}

mkdirSync(lockRoot, { recursive: true });

for (const pkg of nativePackages) {
  const bindingPath = resolve(root, pkg.binding);
  if (existsSync(bindingPath)) {
    continue;
  }

  const lockPath = lockPathFor(pkg);
  // Retry loop handles the rare case where the lock holder fails to produce
  // the binding (e.g., crash or stale lock) and we need to re-acquire.
  let attempts = 0;
  while (!existsSync(bindingPath)) {
    attempts += 1;
    if (attempts > 3) {
      throw new Error(
        `Failed to build NAPI bindings required by Vitest for ${pkg.name} after ${attempts - 1} attempts.`,
      );
    }

    if (tryAcquireLock(lockPath)) {
      try {
        buildPackage(pkg);
      } finally {
        try {
          rmSync(lockPath, { recursive: true, force: true });
        } catch {
          // best-effort cleanup
        }
      }
    } else {
      // Another worker is building; wait until the binding appears or the
      // lock goes stale (in which case we'll retry and acquire).
      waitForBinding(pkg, lockPath);
    }
  }
}
