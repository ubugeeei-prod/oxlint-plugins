import { readFileSync } from 'node:fs';

const inputPath = process.argv[2] ?? 'bench-output.txt';
const ansiPattern = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, 'g');
const raw = readFileSync(inputPath, 'utf8').replace(ansiPattern, '').trim();
const clipped = raw.length > 60_000 ? raw.slice(raw.length - 60_000) : raw;
const summary = extractSummary(clipped);

console.log(`<!-- oxlint-plugins-benchmark-comment -->
## Benchmark Results

${summary}

<details>
<summary>Raw benchmark output</summary>

\`\`\`text
${clipped}
\`\`\`

</details>
`);

function extractSummary(output: string): string {
  const lines = output
    .split('\n')
    .map((line) => line.trimEnd())
    .filter(Boolean);
  const benchLines = lines.filter(
    (line) => line.includes('scan_file_') || line.includes('benches/'),
  );

  if (benchLines.length === 0) {
    return 'Benchmark completed. Expand the raw output for details.';
  }

  return `\`\`\`text\n${benchLines.slice(-24).join('\n')}\n\`\`\``;
}
