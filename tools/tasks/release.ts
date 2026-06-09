import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

type BumpKind = 'major' | 'minor' | 'patch';
type PackageJson = {
  version: string;
};
type StatusPackage = {
  version: string;
};
type JsonVersionOptions = {
  list?: boolean;
};

const rawBump = process.argv[2];
const allowed = new Set<string>(['major', 'minor', 'patch']);

if (!rawBump || !allowed.has(rawBump)) {
  console.error('Usage: vp run release <major|minor|patch>');
  process.exit(1);
}

const bump = rawBump as BumpKind;
ensureCleanGit();

const packageFiles: string[] = [
  'package.json',
  'npm/type-aware/package.json',
  'npm/no-forbidden-identifiers/package.json',
  'npm/stylistic/package.json',
];
const currentVersion = JSON.parse(
  readFileSync('npm/no-forbidden-identifiers/package.json', 'utf8'),
) as PackageJson;
const currentVersionString = currentVersion.version;
const nextVersion = bumpVersion(currentVersionString, bump);

for (const file of packageFiles) {
  updateJsonVersion(file, nextVersion);
}

updateCargoVersion('Cargo.toml', nextVersion);
updateCargoWorkspacePathDependencyVersion('Cargo.toml', 'oxlint-plugins-carton', nextVersion);
updateCargoWorkspacePathDependencyVersion('Cargo.toml', 'oxlint-plugins-stylistic', nextVersion);
updateJsonVersion('status.json', nextVersion, { list: true });
updatePluginRuntimeVersion('status.json', nextVersion);
updateCargoVersion('npm/no-forbidden-identifiers/Cargo.toml', nextVersion);
updateCargoVersion('npm/stylistic/Cargo.toml', nextVersion);
updatePluginRuntimeVersion('npm/no-forbidden-identifiers/index.js', nextVersion);
updatePluginRuntimeVersion('npm/stylistic/index.js', nextVersion);

execFileSync('pnpm', ['install', '--lockfile-only'], { stdio: 'inherit' });
execFileSync('pnpm', ['run', 'verify'], { stdio: 'inherit' });

execFileSync(
  'git',
  [
    'add',
    ...packageFiles,
    'Cargo.toml',
    'status.json',
    'npm/no-forbidden-identifiers/Cargo.toml',
    'npm/no-forbidden-identifiers/index.js',
    'npm/stylistic/Cargo.toml',
    'npm/stylistic/index.js',
    'pnpm-lock.yaml',
  ],
  {
    stdio: 'inherit',
  },
);
execFileSync('git', ['commit', '-m', `chore(release): v${nextVersion}`], { stdio: 'inherit' });
execFileSync('git', ['tag', '-a', `v${nextVersion}`, '-m', `v${nextVersion}`], {
  stdio: 'inherit',
});
execFileSync('git', ['push'], { stdio: 'inherit' });
execFileSync('git', ['push', 'origin', `v${nextVersion}`], { stdio: 'inherit' });

console.log(`Released v${nextVersion}. GitHub Actions will publish packages from the tag.`);

function ensureCleanGit(): void {
  const status = execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8' }).trim();
  if (status.length > 0) {
    console.error('Release requires a clean working tree.');
    console.error(status);
    process.exit(1);
  }
}

function bumpVersion(version: string, kind: BumpKind): string {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    throw new Error(`Unsupported semver version: ${version}`);
  }

  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);

  if (kind === 'major') {
    return `${major + 1}.0.0`;
  }
  if (kind === 'minor') {
    return `${major}.${minor + 1}.0`;
  }
  return `${major}.${minor}.${patch + 1}`;
}

function updateJsonVersion(file: string, version: string, options: JsonVersionOptions = {}): void {
  const path = resolve(file);
  if (options.list) {
    const data = JSON.parse(readFileSync(path, 'utf8')) as StatusPackage[];
    for (const plugin of data) {
      plugin.version = version;
    }
    writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
  } else {
    const data = JSON.parse(readFileSync(path, 'utf8')) as PackageJson;
    data.version = version;
    writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
  }
}

function updateCargoVersion(file: string, version: string): void {
  const path = resolve(file);
  if (!existsSync(path)) {
    return;
  }

  const source = readFileSync(path, 'utf8');
  const next = source.replace(/^(version(?:\.workspace)?\s*=\s*)"[^"]+"/m, `$1"${version}"`);
  writeFileSync(path, next);
}

function updateCargoWorkspacePathDependencyVersion(
  file: string,
  dependency: string,
  version: string,
): void {
  const path = resolve(file);
  const source = readFileSync(path, 'utf8');
  const pattern = new RegExp(`(${escapeRegExp(dependency)} = \\{[^}]*version = )"[^"]+"`, 'm');
  const next = source.replace(pattern, `$1"${version}"`);
  writeFileSync(path, next);
}

function updatePluginRuntimeVersion(file: string, version: string): void {
  const path = resolve(file);
  const source = readFileSync(path, 'utf8');
  const next = source.replace(/version: '\d+\.\d+\.\d+'/g, `version: '${version}'`);
  writeFileSync(path, next);
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
