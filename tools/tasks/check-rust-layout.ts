import { readdirSync } from 'node:fs';
import { join } from 'node:path';

const ignoredDirs = new Set(['.git', 'node_modules', 'target']);
const failures: string[] = [];

walk('.');

if (failures.length > 0) {
  console.error('Rust layout policy failures:');
  for (const failure of failures) {
    console.error(`- ${failure}: mod.rs is not allowed; use named module files.`);
  }
  process.exit(1);
}

console.log('Rust layout policy is satisfied.');

function walk(dir: string): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);

    if (entry.isDirectory()) {
      if (!ignoredDirs.has(entry.name)) {
        walk(path);
      }
      continue;
    }

    if (entry.isFile() && entry.name === 'mod.rs') {
      failures.push(path);
    }
  }
}
