import { readFileSync } from 'node:fs';

type PackageJson = {
  version: string;
};

const ref = process.env.GITHUB_REF_NAME ?? '';
const version = ref.startsWith('v') ? ref.slice(1) : ref;

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Release tags must look like v1.2.3. Received: ${ref}`);
  process.exit(1);
}

const packages: string[] = [
  'npm/type-aware/package.json',
  'npm/no-forbidden-identifiers/package.json',
];

for (const file of packages) {
  const pkg = JSON.parse(readFileSync(file, 'utf8')) as PackageJson;
  if (pkg.version !== version) {
    console.error(`${file} version ${pkg.version} does not match tag ${ref}.`);
    process.exit(1);
  }
}

console.log(`Release tag ${ref} matches package versions.`);
