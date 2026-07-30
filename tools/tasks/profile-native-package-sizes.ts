import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

export const NATIVE_SIZE_SCHEMA_VERSION = 1;
export const NATIVE_SIZE_STRATEGY = 'A-per-plugin-native-addon';
export const NATIVE_SIZE_PROFILE_ID = 'all-native-packages';
export const GZIP_LEVEL = 9;

type PackageJson = {
  name?: unknown;
  version?: unknown;
  private?: unknown;
  files?: unknown;
};

export type NativePackageDescriptor = {
  name: string;
  version: string;
  directory: string;
  absoluteDirectory: string;
  binaryFiles: string[];
};

export type BinaryMeasurement = {
  file: string;
  target: string | null;
  rawBytes: number;
  gzipBytes: number;
  sha256: string;
  gzipSha256: string;
};

export type NpmPackEntry = {
  name: string;
  version: string;
  filename: string;
  size: number;
  unpackedSize: number;
  entryCount: number;
};

export type TarballMeasurement = {
  bytes: number;
  unpackedBytes: number;
  fileCount: number;
  sha256: string;
};

export type NativePackageMeasurement = {
  name: string;
  version: string;
  directory: string;
  binaries: BinaryMeasurement[];
  rawNativeBytes: number;
  gzipNativeBytes: number;
  npmTarballBytes: number;
  npmUnpackedBytes: number;
  npmTarballFileCount: number;
  npmTarballSha256: string;
};

export type NativeSizeToolchain = {
  node: string;
  npm: string;
  pnpm: string;
  rustc: string;
  zlib: string;
};

export type NativeSizeReport = {
  schemaVersion: number;
  measurement: 'native-package-size';
  strategy: typeof NATIVE_SIZE_STRATEGY;
  thresholdPolicy: 'measurement-only';
  source: {
    revision: string;
  };
  environment: {
    platform: NodeJS.Platform;
    architecture: string;
    runnerOs: string | null;
    runnerArchitecture: string | null;
    toolchain: NativeSizeToolchain;
  };
  compression: {
    format: 'gzip';
    level: typeof GZIP_LEVEL;
  };
  packaging: {
    command: 'npm pack --ignore-scripts --json';
    lifecycleScripts: false;
  };
  packages: NativePackageMeasurement[];
  commonInstall: {
    id: typeof NATIVE_SIZE_PROFILE_ID;
    description: string;
    packageCount: number;
    binaryCount: number;
    rawNativeBytes: number;
    gzipNativeBytes: number;
    npmTarballBytes: number;
    npmUnpackedBytes: number;
  };
};

export type ProfileOptions = {
  workspaceRoot: string;
  outputDirectory: string;
  sourceRevision?: string;
};

export type ProfileDependencies = {
  discoverPackages?: (workspaceRoot: string) => NativePackageDescriptor[];
  measureBinary?: (path: string) => BinaryMeasurement;
  packPackage?: (packageDirectory: string, destination: string) => TarballMeasurement;
  toolchain?: () => NativeSizeToolchain;
  resolveRevision?: (workspaceRoot: string, explicit?: string) => string;
  platform?: NodeJS.Platform;
  architecture?: string;
  runnerOs?: string | null;
  runnerArchitecture?: string | null;
  temporaryDirectory?: string;
};

export type CliOptions = ProfileOptions & {
  help: boolean;
};

const WORKSPACE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

export function declaresNativeArtifact(files: unknown): boolean {
  return (
    Array.isArray(files) &&
    files.some(
      (entry) =>
        typeof entry === 'string' &&
        (entry === '*.node' || entry.endsWith('/*.node') || entry.endsWith('.node')),
    )
  );
}

export function discoverNativePackages(workspaceRoot: string): NativePackageDescriptor[] {
  const npmDirectory = join(workspaceRoot, 'npm');
  if (!existsSync(npmDirectory)) {
    throw new Error(`Missing npm workspace directory: ${npmDirectory}`);
  }

  const packages: NativePackageDescriptor[] = [];
  const names = new Set<string>();
  const directories = readdirSync(npmDirectory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();

  for (const directoryName of directories) {
    const absoluteDirectory = join(npmDirectory, directoryName);
    const packageJsonPath = join(absoluteDirectory, 'package.json');
    if (!existsSync(packageJsonPath)) {
      continue;
    }

    let packageJson: PackageJson;
    try {
      packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as PackageJson;
    } catch (error) {
      throw new Error(`Cannot parse ${relative(workspaceRoot, packageJsonPath)}: ${String(error)}`);
    }

    if (packageJson.private === true || !declaresNativeArtifact(packageJson.files)) {
      continue;
    }
    if (typeof packageJson.name !== 'string' || packageJson.name.length === 0) {
      throw new Error(`${relative(workspaceRoot, packageJsonPath)} is missing a package name.`);
    }
    if (typeof packageJson.version !== 'string' || packageJson.version.length === 0) {
      throw new Error(`${relative(workspaceRoot, packageJsonPath)} is missing a package version.`);
    }
    if (names.has(packageJson.name)) {
      throw new Error(`Duplicate native package name: ${packageJson.name}`);
    }

    const binaryFiles = readdirSync(absoluteDirectory)
      .filter((entry) => entry.endsWith('.node'))
      .sort();
    if (binaryFiles.length === 0) {
      throw new Error(
        `${packageJson.name} has no built .node artifact in ${relative(
          workspaceRoot,
          absoluteDirectory,
        )}. Run vp build first.`,
      );
    }

    names.add(packageJson.name);
    packages.push({
      name: packageJson.name,
      version: packageJson.version,
      directory: toPosixPath(relative(workspaceRoot, absoluteDirectory)),
      absoluteDirectory,
      binaryFiles,
    });
  }

  if (packages.length === 0) {
    throw new Error('No publishable native npm packages were discovered. Run vp build first.');
  }

  return packages.sort(
    (left, right) =>
      left.name.localeCompare(right.name, 'en') ||
      left.directory.localeCompare(right.directory, 'en'),
  );
}

export function binaryTarget(filename: string): string | null {
  const match = /\.([^.]+)\.node$/u.exec(filename);
  return match?.[1] ?? null;
}

export function measureNativeBinary(path: string): BinaryMeasurement {
  const contents = readFileSync(path);
  const compressed = gzipSync(contents, { level: GZIP_LEVEL });
  return {
    file: basename(path),
    target: binaryTarget(basename(path)),
    rawBytes: contents.byteLength,
    gzipBytes: compressed.byteLength,
    sha256: sha256(contents),
    gzipSha256: sha256(compressed),
  };
}

export function parseNpmPackJson(output: string): NpmPackEntry {
  let parsed: unknown;
  try {
    parsed = JSON.parse(output);
  } catch (error) {
    throw new Error(`npm pack did not return valid JSON: ${String(error)}`);
  }

  const candidates = packCandidates(parsed);
  if (candidates.length !== 1) {
    throw new Error(`Expected exactly one npm pack result, received ${candidates.length}.`);
  }

  const candidate = candidates[0];
  if (!isRecord(candidate)) {
    throw new Error('npm pack result is not an object.');
  }

  const name = requiredString(candidate.name, 'name');
  const version = requiredString(candidate.version, 'version');
  const filename = requiredString(candidate.filename, 'filename');
  const size = requiredNonNegativeInteger(candidate.size, 'size');
  const unpackedSize = requiredNonNegativeInteger(candidate.unpackedSize, 'unpackedSize');
  const files = candidate.files;
  const entryCount =
    candidate.entryCount === undefined
      ? Array.isArray(files)
        ? files.length
        : 0
      : requiredNonNegativeInteger(candidate.entryCount, 'entryCount');

  return { name, version, filename, size, unpackedSize, entryCount };
}

export function resolvePackArtifactPath(destination: string, filename: string): string {
  const destinationRoot = resolve(destination);
  const artifactPath = resolve(
    destinationRoot,
    isAbsolute(filename) ? relative(destinationRoot, filename) : filename,
  );
  const pathFromDestination = relative(destinationRoot, artifactPath);
  if (
    pathFromDestination === '..' ||
    pathFromDestination.startsWith(`..${sep}`) ||
    isAbsolute(pathFromDestination)
  ) {
    throw new Error(`npm pack returned an artifact outside its destination: ${filename}`);
  }
  return artifactPath;
}

export function packNativePackage(
  packageDirectory: string,
  destination: string,
): TarballMeasurement {
  mkdirSync(destination, { recursive: true });
  const output = execFileSync(
    npmCommand(),
    ['pack', '--ignore-scripts', '--json', '--pack-destination', destination],
    {
      cwd: packageDirectory,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  const result = parseNpmPackJson(output);
  const artifactPath = resolvePackArtifactPath(destination, result.filename);
  if (!existsSync(artifactPath)) {
    throw new Error(`npm pack did not create ${artifactPath}.`);
  }

  const contents = readFileSync(artifactPath);
  if (contents.byteLength !== result.size) {
    throw new Error(
      `npm pack size mismatch for ${result.name}: JSON reported ${result.size}, file has ${contents.byteLength}.`,
    );
  }

  return {
    bytes: contents.byteLength,
    unpackedBytes: result.unpackedSize,
    fileCount: result.entryCount,
    sha256: sha256(contents),
  };
}

export function collectToolchainMetadata(): NativeSizeToolchain {
  return {
    node: process.version,
    npm: commandVersion(npmCommand(), ['--version']),
    pnpm: commandVersion(process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm', ['--version']),
    rustc: commandVersion('rustc', ['--version', '--verbose']),
    zlib: process.versions.zlib,
  };
}

export function resolveSourceRevision(workspaceRoot: string, explicit?: string): string {
  const revision = explicit?.trim();
  if (revision) {
    return revision;
  }
  return execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: workspaceRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

export function summarizeCommonInstall(
  packages: NativePackageMeasurement[],
): NativeSizeReport['commonInstall'] {
  return packages.reduce<NativeSizeReport['commonInstall']>(
    (summary, packageMeasurement) => {
      summary.packageCount += 1;
      summary.binaryCount += packageMeasurement.binaries.length;
      summary.rawNativeBytes += packageMeasurement.rawNativeBytes;
      summary.gzipNativeBytes += packageMeasurement.gzipNativeBytes;
      summary.npmTarballBytes += packageMeasurement.npmTarballBytes;
      summary.npmUnpackedBytes += packageMeasurement.npmUnpackedBytes;
      return summary;
    },
    {
      id: NATIVE_SIZE_PROFILE_ID,
      description:
        'Strategy A upper-bound/meta-equivalent install: every publishable npm package that ships its own native addon on this platform.',
      packageCount: 0,
      binaryCount: 0,
      rawNativeBytes: 0,
      gzipNativeBytes: 0,
      npmTarballBytes: 0,
      npmUnpackedBytes: 0,
    },
  );
}

export function buildNativeSizeReport(
  packages: NativePackageMeasurement[],
  metadata: {
    revision: string;
    platform: NodeJS.Platform;
    architecture: string;
    runnerOs: string | null;
    runnerArchitecture: string | null;
    toolchain: NativeSizeToolchain;
  },
): NativeSizeReport {
  const sortedPackages = [...packages].sort(
    (left, right) =>
      left.name.localeCompare(right.name, 'en') ||
      left.directory.localeCompare(right.directory, 'en'),
  );
  return {
    schemaVersion: NATIVE_SIZE_SCHEMA_VERSION,
    measurement: 'native-package-size',
    strategy: NATIVE_SIZE_STRATEGY,
    thresholdPolicy: 'measurement-only',
    source: { revision: metadata.revision },
    environment: {
      platform: metadata.platform,
      architecture: metadata.architecture,
      runnerOs: metadata.runnerOs,
      runnerArchitecture: metadata.runnerArchitecture,
      toolchain: metadata.toolchain,
    },
    compression: {
      format: 'gzip',
      level: GZIP_LEVEL,
    },
    packaging: {
      command: 'npm pack --ignore-scripts --json',
      lifecycleScripts: false,
    },
    packages: sortedPackages,
    commonInstall: summarizeCommonInstall(sortedPackages),
  };
}

export function profileNativePackageSizes(
  options: ProfileOptions,
  dependencies: ProfileDependencies = {},
): NativeSizeReport {
  const discoverPackages = dependencies.discoverPackages ?? discoverNativePackages;
  const measureBinary = dependencies.measureBinary ?? measureNativeBinary;
  const packPackage = dependencies.packPackage ?? packNativePackage;
  const descriptors = discoverPackages(options.workspaceRoot);
  const temporaryDirectory =
    dependencies.temporaryDirectory ??
    mkdtempSync(join(tmpdir(), 'oxlint-plugins-native-package-sizes-'));
  const ownsTemporaryDirectory = dependencies.temporaryDirectory === undefined;

  try {
    const packages = descriptors.map((descriptor, index) => {
      const binaries = descriptor.binaryFiles
        .map((filename) => measureBinary(join(descriptor.absoluteDirectory, filename)))
        .sort((left, right) => left.file.localeCompare(right.file, 'en'));
      const tarball = packPackage(
        descriptor.absoluteDirectory,
        join(temporaryDirectory, String(index).padStart(3, '0')),
      );
      return {
        name: descriptor.name,
        version: descriptor.version,
        directory: descriptor.directory,
        binaries,
        rawNativeBytes: sum(binaries.map((binary) => binary.rawBytes)),
        gzipNativeBytes: sum(binaries.map((binary) => binary.gzipBytes)),
        npmTarballBytes: tarball.bytes,
        npmUnpackedBytes: tarball.unpackedBytes,
        npmTarballFileCount: tarball.fileCount,
        npmTarballSha256: tarball.sha256,
      };
    });

    return buildNativeSizeReport(packages, {
      revision: (dependencies.resolveRevision ?? resolveSourceRevision)(
        options.workspaceRoot,
        options.sourceRevision,
      ),
      platform: dependencies.platform ?? process.platform,
      architecture: dependencies.architecture ?? process.arch,
      runnerOs: dependencies.runnerOs ?? process.env.RUNNER_OS ?? null,
      runnerArchitecture: dependencies.runnerArchitecture ?? process.env.RUNNER_ARCH ?? null,
      toolchain: (dependencies.toolchain ?? collectToolchainMetadata)(),
    });
  } finally {
    if (ownsTemporaryDirectory) {
      rmSync(temporaryDirectory, { recursive: true, force: true });
    }
  }
}

export function renderNativeSizeMarkdown(report: NativeSizeReport): string {
  const lines = [
    '# Native package size profile',
    '',
    `- Strategy: \`${report.strategy}\``,
    `- Source revision: \`${report.source.revision}\``,
    `- Environment: \`${report.environment.platform}/${report.environment.architecture}\``,
    `- Node/npm/pnpm: \`${firstLine(report.environment.toolchain.node)}\` / \`${firstLine(
      report.environment.toolchain.npm,
    )}\` / \`${firstLine(report.environment.toolchain.pnpm)}\``,
    `- Compression: \`${report.compression.format}\` level \`${report.compression.level}\``,
    '- Policy: measurement only; this report intentionally has no pass/fail size threshold.',
    '',
    '| Package | Native binaries | Raw .node | gzip .node | npm tarball | npm unpacked |',
    '| --- | ---: | ---: | ---: | ---: | ---: |',
    ...report.packages.map(
      (packageMeasurement) =>
        `| ${escapeMarkdownCell(packageMeasurement.name)} | ${
          packageMeasurement.binaries.length
        } | ${formatBytes(packageMeasurement.rawNativeBytes)} | ${formatBytes(
          packageMeasurement.gzipNativeBytes,
        )} | ${formatBytes(packageMeasurement.npmTarballBytes)} | ${formatBytes(
          packageMeasurement.npmUnpackedBytes,
        )} |`,
    ),
    '',
    `## Common install: ${report.commonInstall.id}`,
    '',
    report.commonInstall.description,
    '',
    `- Packages: ${report.commonInstall.packageCount}`,
    `- Native binaries: ${report.commonInstall.binaryCount}`,
    `- Raw native total: ${formatBytes(report.commonInstall.rawNativeBytes)}`,
    `- gzip native total: ${formatBytes(report.commonInstall.gzipNativeBytes)}`,
    `- npm tarball total: ${formatBytes(report.commonInstall.npmTarballBytes)}`,
    `- npm unpacked total: ${formatBytes(report.commonInstall.npmUnpackedBytes)}`,
    '',
  ];
  return lines.join('\n');
}

export function writeNativeSizeArtifacts(
  outputDirectory: string,
  report: NativeSizeReport,
): { jsonPath: string; markdownPath: string } {
  mkdirSync(outputDirectory, { recursive: true });
  const jsonPath = join(outputDirectory, 'native-package-sizes.json');
  const markdownPath = join(outputDirectory, 'native-package-sizes.md');
  writeFileSync(jsonPath, `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(markdownPath, renderNativeSizeMarkdown(report));
  return { jsonPath, markdownPath };
}

export function formatBytes(bytes: number): string {
  if (!Number.isSafeInteger(bytes) || bytes < 0) {
    throw new Error(`Byte count must be a non-negative safe integer: ${bytes}`);
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(2)} ${unit}`;
}

export function parseCliArguments(
  arguments_: string[],
  currentWorkingDirectory = process.cwd(),
): CliOptions {
  const options: CliOptions = {
    workspaceRoot: WORKSPACE_ROOT,
    outputDirectory: join(WORKSPACE_ROOT, 'profiles', 'native-package-sizes'),
    help: false,
  };

  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === '--') {
      continue;
    }
    if (argument === '--help' || argument === '-h') {
      options.help = true;
      continue;
    }
    const [name, inlineValue] = splitArgument(argument);
    if (!['--workspace-root', '--output-dir', '--source-revision'].includes(name)) {
      throw new Error(`Unknown argument: ${argument}`);
    }
    const value = inlineValue ?? arguments_[++index];
    if (value === undefined || value.startsWith('--')) {
      throw new Error(`${name} requires a value.`);
    }
    if (name === '--workspace-root') {
      options.workspaceRoot = resolve(currentWorkingDirectory, value);
    } else if (name === '--output-dir') {
      options.outputDirectory = resolve(currentWorkingDirectory, value);
    } else {
      const revision = value.trim();
      if (revision.length === 0) {
        throw new Error('--source-revision requires a non-empty value.');
      }
      options.sourceRevision = revision;
    }
  }
  return options;
}

export function usage(): string {
  return [
    'Usage: pnpm run profile:native-sizes -- [options]',
    '',
    'Options:',
    '  --workspace-root <path>    Workspace root containing npm/* packages.',
    '  --output-dir <path>        Directory for JSON and Markdown artifacts.',
    '  --source-revision <sha>    Revision recorded in comparable metadata.',
    '  -h, --help                 Show this help.',
    '',
  ].join('\n');
}

function packCandidates(parsed: unknown): unknown[] {
  if (Array.isArray(parsed)) {
    return parsed;
  }
  if (!isRecord(parsed)) {
    return [];
  }
  if (typeof parsed.name === 'string') {
    return [parsed];
  }
  return Object.values(parsed).filter((value) => isRecord(value) && typeof value.name === 'string');
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`npm pack result has an invalid ${field}.`);
  }
  return value;
}

function requiredNonNegativeInteger(value: unknown, field: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`npm pack result has an invalid ${field}.`);
  }
  return value as number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function sha256(contents: NodeJS.ArrayBufferView): string {
  return createHash('sha256').update(contents).digest('hex');
}

function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

function commandVersion(command: string, arguments_: string[]): string {
  return execFileSync(command, arguments_, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

function npmCommand(): string {
  return process.platform === 'win32' ? 'npm.cmd' : 'npm';
}

function toPosixPath(path: string): string {
  return path.split(sep).join('/');
}

function firstLine(value: string): string {
  return value.split(/\r?\n/u, 1)[0];
}

function escapeMarkdownCell(value: string): string {
  return value.replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function splitArgument(argument: string): [string, string | undefined] {
  const equals = argument.indexOf('=');
  return equals === -1
    ? [argument, undefined]
    : [argument.slice(0, equals), argument.slice(equals + 1)];
}

function isDirectExecution(): boolean {
  return (
    process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)
  );
}

if (isDirectExecution()) {
  try {
    const options = parseCliArguments(process.argv.slice(2));
    if (options.help) {
      process.stdout.write(usage());
    } else {
      const report = profileNativePackageSizes({
        ...options,
        sourceRevision: options.sourceRevision ?? process.env.NATIVE_SIZE_SOURCE_REVISION,
      });
      const artifacts = writeNativeSizeArtifacts(options.outputDirectory, report);
      process.stdout.write(
        `${renderNativeSizeMarkdown(report)}\nArtifacts:\n- ${artifacts.jsonPath}\n- ${
          artifacts.markdownPath
        }\n`,
      );
    }
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
