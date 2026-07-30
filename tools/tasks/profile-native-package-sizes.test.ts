import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  GZIP_LEVEL,
  NATIVE_SIZE_PROFILE_ID,
  NATIVE_SIZE_SCHEMA_VERSION,
  NATIVE_SIZE_STRATEGY,
  binaryTarget,
  buildNativeSizeReport,
  declaresNativeArtifact,
  discoverNativePackages,
  formatBytes,
  measureNativeBinary,
  packNativePackage,
  parseCliArguments,
  parseNpmPackJson,
  profileNativePackageSizes,
  renderNativeSizeMarkdown,
  resolvePackArtifactPath,
  resolveSourceRevision,
  summarizeCommonInstall,
  usage,
  writeNativeSizeArtifacts,
  type NativePackageMeasurement,
  type NativeSizeToolchain,
} from './profile-native-package-sizes.js';

const temporaryDirectories: string[] = [];
const toolchain: NativeSizeToolchain = {
  node: 'v24.0.0',
  npm: '11.0.0',
  pnpm: '11.5.2',
  rustc: 'rustc 1.96.0\ncommit-hash: example',
  zlib: '1.3.1',
};

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

describe('declaresNativeArtifact', () => {
  it.each([
    [['*.node'], true],
    [['index.js', '*.node'], true],
    [['dist/*.node'], true],
    [['native.darwin-arm64.node'], true],
    [['native.js'], false],
    [[], false],
    [undefined, false],
    ['*.node', false],
  ])('classifies %j as %s', (files, expected) => {
    expect(declaresNativeArtifact(files)).toBe(expected);
  });
});

describe('discoverNativePackages', () => {
  it('finds publishable packages with built native artifacts in stable name order', () => {
    const root = workspace();
    writePackage(root, 'zeta', {
      name: '@scope/zeta',
      version: '2.0.0',
      files: ['index.js', '*.node'],
      binaries: ['zeta.linux-x64-gnu.node', 'zeta.darwin-arm64.node'],
    });
    writePackage(root, 'alpha', {
      name: '@scope/alpha',
      version: '1.0.0',
      files: ['*.node'],
      binaries: ['alpha.linux-x64-gnu.node'],
    });

    expect(discoverNativePackages(root)).toMatchObject([
      {
        name: '@scope/alpha',
        version: '1.0.0',
        directory: 'npm/alpha',
        binaryFiles: ['alpha.linux-x64-gnu.node'],
      },
      {
        name: '@scope/zeta',
        version: '2.0.0',
        directory: 'npm/zeta',
        binaryFiles: ['zeta.darwin-arm64.node', 'zeta.linux-x64-gnu.node'],
      },
    ]);
  });

  it('ignores non-package directories, non-native packages, and private packages', () => {
    const root = workspace();
    mkdirSync(join(root, 'npm', 'empty'));
    writePackage(root, 'javascript', {
      name: '@scope/javascript',
      version: '1.0.0',
      files: ['index.js'],
    });
    writePackage(root, 'private-native', {
      name: '@scope/private-native',
      version: '1.0.0',
      private: true,
      files: ['*.node'],
      binaries: ['private.node'],
    });
    writePackage(root, 'public-native', {
      name: '@scope/public-native',
      version: '1.0.0',
      files: ['*.node'],
      binaries: ['public.node'],
    });

    expect(discoverNativePackages(root).map((entry) => entry.name)).toEqual([
      '@scope/public-native',
    ]);
  });

  it('rejects a missing npm workspace', () => {
    const root = temporaryDirectory();
    expect(() => discoverNativePackages(root)).toThrow('Missing npm workspace directory');
  });

  it('rejects malformed package JSON with its relative path', () => {
    const root = workspace();
    const directory = join(root, 'npm', 'broken');
    mkdirSync(directory);
    writeFileSync(join(directory, 'package.json'), '{');
    expect(() => discoverNativePackages(root)).toThrow('Cannot parse npm/broken/package.json');
  });

  it('rejects a native package without a name', () => {
    const root = workspace();
    writePackage(root, 'missing-name', {
      version: '1.0.0',
      files: ['*.node'],
      binaries: ['native.node'],
    });
    expect(() => discoverNativePackages(root)).toThrow('missing a package name');
  });

  it('rejects a native package without a version', () => {
    const root = workspace();
    writePackage(root, 'missing-version', {
      name: '@scope/missing-version',
      files: ['*.node'],
      binaries: ['native.node'],
    });
    expect(() => discoverNativePackages(root)).toThrow('missing a package version');
  });

  it('rejects duplicate native package names', () => {
    const root = workspace();
    for (const directory of ['one', 'two']) {
      writePackage(root, directory, {
        name: '@scope/duplicate',
        version: '1.0.0',
        files: ['*.node'],
        binaries: [`${directory}.node`],
      });
    }
    expect(() => discoverNativePackages(root)).toThrow(
      'Duplicate native package name: @scope/duplicate',
    );
  });

  it('requires a built artifact for every selected package', () => {
    const root = workspace();
    writePackage(root, 'not-built', {
      name: '@scope/not-built',
      version: '1.0.0',
      files: ['*.node'],
    });
    expect(() => discoverNativePackages(root)).toThrow(
      '@scope/not-built has no built .node artifact',
    );
  });

  it('rejects an npm workspace without publishable native packages', () => {
    const root = workspace();
    writePackage(root, 'javascript', {
      name: '@scope/javascript',
      version: '1.0.0',
      files: ['index.js'],
    });
    expect(() => discoverNativePackages(root)).toThrow(
      'No publishable native npm packages were discovered',
    );
  });
});

describe('native binary measurement', () => {
  it.each([
    ['binding.darwin-arm64.node', 'darwin-arm64'],
    ['binding.linux-x64-gnu.node', 'linux-x64-gnu'],
    ['binding.win32-x64-msvc.node', 'win32-x64-msvc'],
    ['binding.node', null],
    ['binding.js', null],
  ])('extracts the target from %s', (filename, target) => {
    expect(binaryTarget(filename)).toBe(target);
  });

  it('records exact raw bytes, deterministic gzip bytes, and hashes', () => {
    const directory = temporaryDirectory();
    const path = join(directory, 'binding.linux-x64-gnu.node');
    writeFileSync(path, Buffer.from('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'));

    const first = measureNativeBinary(path);
    const second = measureNativeBinary(path);

    expect(first).toEqual(second);
    expect(first.file).toBe('binding.linux-x64-gnu.node');
    expect(first.target).toBe('linux-x64-gnu');
    expect(first.rawBytes).toBe(32);
    expect(first.gzipBytes).toBeGreaterThan(0);
    expect(first.gzipBytes).toBeLessThan(first.rawBytes);
    expect(first.sha256).toMatch(/^[a-f0-9]{64}$/u);
    expect(first.gzipSha256).toMatch(/^[a-f0-9]{64}$/u);
  });

  it('supports an empty native artifact without inventing bytes', () => {
    const directory = temporaryDirectory();
    const path = join(directory, 'empty.node');
    writeFileSync(path, Buffer.alloc(0));
    const measurement = measureNativeBinary(path);
    expect(measurement.rawBytes).toBe(0);
    expect(measurement.gzipBytes).toBeGreaterThan(0);
  });
});

describe('parseNpmPackJson', () => {
  const entry = {
    name: '@scope/plugin',
    version: '1.2.3',
    filename: 'scope-plugin-1.2.3.tgz',
    size: 123,
    unpackedSize: 456,
    entryCount: 7,
    files: [{ path: 'index.js' }],
  };

  it('parses the npm 12 object keyed by package name', () => {
    expect(parseNpmPackJson(JSON.stringify({ '@scope/plugin': entry }))).toEqual({
      name: '@scope/plugin',
      version: '1.2.3',
      filename: 'scope-plugin-1.2.3.tgz',
      size: 123,
      unpackedSize: 456,
      entryCount: 7,
    });
  });

  it('parses npm array output', () => {
    expect(parseNpmPackJson(JSON.stringify([entry])).name).toBe('@scope/plugin');
  });

  it('parses a direct pack result object', () => {
    expect(parseNpmPackJson(JSON.stringify(entry)).version).toBe('1.2.3');
  });

  it('uses the files array when entryCount is absent', () => {
    const { entryCount: _, ...withoutCount } = entry;
    expect(
      parseNpmPackJson(
        JSON.stringify({ ...withoutCount, files: [{ path: 'one' }, { path: 'two' }] }),
      ).entryCount,
    ).toBe(2);
  });

  it('uses zero when neither entryCount nor files are available', () => {
    const { entryCount: _, files: __, ...minimal } = entry;
    expect(parseNpmPackJson(JSON.stringify(minimal)).entryCount).toBe(0);
  });

  it('rejects invalid JSON', () => {
    expect(() => parseNpmPackJson('{')).toThrow('did not return valid JSON');
  });

  it('rejects zero and multiple pack results', () => {
    expect(() => parseNpmPackJson('{}')).toThrow('received 0');
    expect(() => parseNpmPackJson(JSON.stringify([entry, entry]))).toThrow('received 2');
  });

  it.each([
    [{ ...entry, name: '' }, 'name'],
    [{ ...entry, version: null }, 'version'],
    [{ ...entry, filename: 1 }, 'filename'],
    [{ ...entry, size: -1 }, 'size'],
    [{ ...entry, size: 1.5 }, 'size'],
    [{ ...entry, unpackedSize: Number.MAX_SAFE_INTEGER + 1 }, 'unpackedSize'],
    [{ ...entry, entryCount: -1 }, 'entryCount'],
  ])('rejects invalid %s metadata', (candidate, field) => {
    expect(() => parseNpmPackJson(JSON.stringify(candidate))).toThrow(`invalid ${String(field)}`);
  });
});

describe('npm tarball measurement', () => {
  it('keeps artifacts inside the requested destination', () => {
    const destination = temporaryDirectory();
    expect(resolvePackArtifactPath(destination, 'plugin.tgz')).toBe(
      join(destination, 'plugin.tgz'),
    );
    expect(resolvePackArtifactPath(destination, join(destination, 'plugin.tgz'))).toBe(
      join(destination, 'plugin.tgz'),
    );
  });

  it('rejects relative and absolute traversal outside the destination', () => {
    const destination = temporaryDirectory();
    expect(() => resolvePackArtifactPath(destination, '../plugin.tgz')).toThrow(
      'outside its destination',
    );
    expect(() =>
      resolvePackArtifactPath(destination, resolve(destination, '..', 'plugin.tgz')),
    ).toThrow('outside its destination');
  });

  it('packs a built package without running lifecycle scripts', () => {
    const root = temporaryDirectory();
    const packageDirectory = join(root, 'package');
    const destination = join(root, 'artifacts');
    mkdirSync(packageDirectory);
    writeFileSync(join(packageDirectory, 'binding.node'), 'native');
    writeFileSync(join(packageDirectory, 'index.js'), 'export default true;\n');
    writeFileSync(
      join(packageDirectory, 'package.json'),
      `${JSON.stringify(
        {
          name: '@scope/pack-fixture',
          version: '1.0.0',
          files: ['binding.node', 'index.js'],
          scripts: {
            prepack: "node -e \"require('node:fs').writeFileSync('prepack-ran', 'yes')\"",
          },
        },
        null,
        2,
      )}\n`,
    );

    const first = packNativePackage(packageDirectory, join(destination, 'first'));
    const second = packNativePackage(packageDirectory, join(destination, 'second'));

    expect(first).toEqual(second);
    expect(first.bytes).toBeGreaterThan(0);
    expect(first.unpackedBytes).toBeGreaterThanOrEqual('native'.length);
    expect(first.fileCount).toBeGreaterThanOrEqual(3);
    expect(first.sha256).toMatch(/^[a-f0-9]{64}$/u);
    expect(() => readFileSync(join(packageDirectory, 'prepack-ran'))).toThrow();
  });
});

describe('report construction', () => {
  it('sums every common-install size axis independently', () => {
    const packages = [packageMeasurement('b', 10), packageMeasurement('a', 20)];
    const summary = summarizeCommonInstall(packages);
    expect(summary).toMatchObject({
      id: NATIVE_SIZE_PROFILE_ID,
      packageCount: 2,
      binaryCount: 2,
      rawNativeBytes: 30,
      gzipNativeBytes: 15,
      npmTarballBytes: 90,
      npmUnpackedBytes: 120,
    });
  });

  it('returns explicit zero totals for an empty package list', () => {
    expect(summarizeCommonInstall([])).toMatchObject({
      packageCount: 0,
      binaryCount: 0,
      rawNativeBytes: 0,
      gzipNativeBytes: 0,
      npmTarballBytes: 0,
      npmUnpackedBytes: 0,
    });
  });

  it('sorts packages and records comparable measurement metadata without a timestamp', () => {
    const report = buildNativeSizeReport(
      [packageMeasurement('zeta', 2), packageMeasurement('alpha', 1)],
      {
        revision: 'abc123',
        platform: 'linux',
        architecture: 'x64',
        runnerOs: 'Linux',
        runnerArchitecture: 'X64',
        toolchain,
      },
    );

    expect(report.schemaVersion).toBe(NATIVE_SIZE_SCHEMA_VERSION);
    expect(report.strategy).toBe(NATIVE_SIZE_STRATEGY);
    expect(report.thresholdPolicy).toBe('measurement-only');
    expect(report.packages.map((entry) => entry.name)).toEqual(['@scope/alpha', '@scope/zeta']);
    expect(report.compression).toEqual({ format: 'gzip', level: GZIP_LEVEL });
    expect(report.packaging.lifecycleScripts).toBe(false);
    expect(JSON.stringify(report)).not.toMatch(/generatedAt|timestamp|thresholdBytes/u);
  });

  it('profiles injected packages deterministically and uses separate pack destinations', () => {
    const root = temporaryDirectory();
    const packageA = join(root, 'npm', 'a');
    const packageB = join(root, 'npm', 'b');
    mkdirSync(packageA, { recursive: true });
    mkdirSync(packageB, { recursive: true });
    const destinations: string[] = [];

    const report = profileNativePackageSizes(
      {
        workspaceRoot: root,
        outputDirectory: join(root, 'output'),
        sourceRevision: 'explicit-sha',
      },
      {
        discoverPackages: () => [
          {
            name: '@scope/b',
            version: '1.0.0',
            directory: 'npm/b',
            absoluteDirectory: packageB,
            binaryFiles: ['b.node'],
          },
          {
            name: '@scope/a',
            version: '1.0.0',
            directory: 'npm/a',
            absoluteDirectory: packageA,
            binaryFiles: ['a.node'],
          },
        ],
        measureBinary: (path) => ({
          file: path.endsWith('a.node') ? 'a.node' : 'b.node',
          target: null,
          rawBytes: path.endsWith('a.node') ? 1 : 2,
          gzipBytes: 1,
          sha256: 'a'.repeat(64),
          gzipSha256: 'b'.repeat(64),
        }),
        packPackage: (_packageDirectory, destination) => {
          destinations.push(destination);
          return {
            bytes: 3,
            unpackedBytes: 4,
            fileCount: 2,
            sha256: 'c'.repeat(64),
          };
        },
        toolchain: () => toolchain,
        resolveRevision: (_workspaceRoot, explicit) => explicit ?? 'fallback',
        platform: 'linux',
        architecture: 'x64',
        runnerOs: 'Linux',
        runnerArchitecture: 'X64',
        temporaryDirectory: join(root, 'temporary'),
      },
    );

    expect(report.source.revision).toBe('explicit-sha');
    expect(report.packages.map((entry) => entry.name)).toEqual(['@scope/a', '@scope/b']);
    expect(report.commonInstall).toMatchObject({
      rawNativeBytes: 3,
      gzipNativeBytes: 2,
      npmTarballBytes: 6,
      npmUnpackedBytes: 8,
    });
    expect(destinations).toHaveLength(2);
    expect(new Set(destinations).size).toBe(2);
  });
});

describe('Markdown and artifact output', () => {
  it('renders the strategy, comparability metadata, table, totals, and no-threshold policy', () => {
    const report = buildNativeSizeReport([packageMeasurement('pipe|name', 2048)], {
      revision: 'deadbeef',
      platform: 'darwin',
      architecture: 'arm64',
      runnerOs: null,
      runnerArchitecture: null,
      toolchain,
    });
    const markdown = renderNativeSizeMarkdown(report);

    expect(markdown).toContain('# Native package size profile');
    expect(markdown).toContain(`\`${NATIVE_SIZE_STRATEGY}\``);
    expect(markdown).toContain('`deadbeef`');
    expect(markdown).toContain('`darwin/arm64`');
    expect(markdown).toContain('measurement only');
    expect(markdown).toContain('@scope/pipe\\|name');
    expect(markdown).toContain('2.00 KiB');
    expect(markdown).toContain(`## Common install: ${NATIVE_SIZE_PROFILE_ID}`);
    expect(markdown.endsWith('\n')).toBe(true);
  });

  it('writes stable newline-terminated JSON and Markdown artifacts', () => {
    const directory = join(temporaryDirectory(), 'nested', 'output');
    const report = buildNativeSizeReport([packageMeasurement('plugin', 1)], {
      revision: 'revision',
      platform: 'linux',
      architecture: 'x64',
      runnerOs: 'Linux',
      runnerArchitecture: 'X64',
      toolchain,
    });
    const paths = writeNativeSizeArtifacts(directory, report);
    const json = readFileSync(paths.jsonPath, 'utf8');
    const markdown = readFileSync(paths.markdownPath, 'utf8');

    expect(json.endsWith('\n')).toBe(true);
    expect(JSON.parse(json)).toEqual(report);
    expect(markdown).toBe(renderNativeSizeMarkdown(report));
    expect(markdown.endsWith('\n')).toBe(true);
  });
});

describe('formatBytes', () => {
  it.each([
    [0, '0 B'],
    [1, '1 B'],
    [1023, '1023 B'],
    [1024, '1.00 KiB'],
    [1536, '1.50 KiB'],
    [1024 ** 2, '1.00 MiB'],
    [1024 ** 3, '1.00 GiB'],
    [1024 ** 4, '1.00 TiB'],
  ])('formats %d as %s', (bytes, expected) => {
    expect(formatBytes(bytes)).toBe(expected);
  });

  it.each([-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    'rejects invalid byte count %s',
    (bytes) => {
      expect(() => formatBytes(bytes)).toThrow('non-negative safe integer');
    },
  );
});

describe('revision and CLI handling', () => {
  it('prefers a trimmed explicit revision', () => {
    expect(resolveSourceRevision('/not/used', '  explicit  ')).toBe('explicit');
  });

  it('reads the current Git revision when no override is supplied', () => {
    expect(resolveSourceRevision(resolve(import.meta.dirname, '..', '..'))).toMatch(
      /^[a-f0-9]{40}$/u,
    );
  });

  it('provides workspace-relative defaults', () => {
    const options = parseCliArguments([], '/tmp/current');
    expect(options.help).toBe(false);
    expect(options.workspaceRoot).toBe(resolve(import.meta.dirname, '..', '..'));
    expect(options.outputDirectory).toBe(
      join(options.workspaceRoot, 'profiles', 'native-package-sizes'),
    );
  });

  it('parses spaced and inline argument values', () => {
    const options = parseCliArguments(
      [
        '--workspace-root',
        './workspace',
        '--output-dir=./artifacts',
        '--source-revision',
        'abc123',
      ],
      '/tmp/current',
    );
    expect(options).toMatchObject({
      workspaceRoot: '/tmp/current/workspace',
      outputDirectory: '/tmp/current/artifacts',
      sourceRevision: 'abc123',
    });
  });

  it('accepts the package-manager option separator', () => {
    expect(parseCliArguments(['--', '--source-revision', 'abc123']).sourceRevision).toBe('abc123');
  });

  it.each(['--help', '-h'])('recognizes %s', (argument) => {
    expect(parseCliArguments([argument]).help).toBe(true);
  });

  it('rejects unknown arguments', () => {
    expect(() => parseCliArguments(['--threshold', '1'])).toThrow('Unknown argument: --threshold');
  });

  it.each(['--workspace-root', '--output-dir', '--source-revision'])(
    'requires a value for %s',
    (argument) => {
      expect(() => parseCliArguments([argument])).toThrow(`${argument} requires a value`);
      expect(() => parseCliArguments([argument, '--help'])).toThrow(`${argument} requires a value`);
    },
  );

  it('rejects an empty source revision', () => {
    expect(() => parseCliArguments(['--source-revision='])).toThrow('non-empty value');
  });

  it('documents measurement options without exposing a threshold option', () => {
    expect(usage()).toContain('profile:native-sizes');
    expect(usage()).toContain('--source-revision');
    expect(usage()).not.toContain('--threshold');
    expect(usage().endsWith('\n')).toBe(true);
  });
});

function temporaryDirectory(): string {
  const directory = mkdtempSync(join(tmpdir(), 'native-package-sizes-test-'));
  temporaryDirectories.push(directory);
  return directory;
}

function workspace(): string {
  const root = temporaryDirectory();
  mkdirSync(join(root, 'npm'));
  return root;
}

function writePackage(
  root: string,
  directoryName: string,
  options: {
    name?: string;
    version?: string;
    private?: boolean;
    files?: string[];
    binaries?: string[];
  },
): void {
  const directory = join(root, 'npm', directoryName);
  mkdirSync(directory);
  const { binaries = [], ...packageJson } = options;
  writeFileSync(join(directory, 'package.json'), `${JSON.stringify(packageJson, null, 2)}\n`);
  for (const binary of binaries) {
    writeFileSync(join(directory, binary), `binary:${binary}`);
  }
}

function packageMeasurement(name: string, rawBytes: number): NativePackageMeasurement {
  return {
    name: `@scope/${name}`,
    version: '1.0.0',
    directory: `npm/${name}`,
    binaries: [
      {
        file: `${name}.linux-x64-gnu.node`,
        target: 'linux-x64-gnu',
        rawBytes,
        gzipBytes: Math.floor(rawBytes / 2),
        sha256: 'a'.repeat(64),
        gzipSha256: 'b'.repeat(64),
      },
    ],
    rawNativeBytes: rawBytes,
    gzipNativeBytes: Math.floor(rawBytes / 2),
    npmTarballBytes: rawBytes * 3,
    npmUnpackedBytes: rawBytes * 4,
    npmTarballFileCount: 5,
    npmTarballSha256: 'c'.repeat(64),
  };
}
