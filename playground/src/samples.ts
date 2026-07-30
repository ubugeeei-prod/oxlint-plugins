// Starter snippets that each surface a few diagnostics, so the playground shows
// something useful the moment it loads.
export type Sample = {
  label: string;
  filename: string;
  code: string;
};

export const samples: Sample[] = [
  {
    label: 'Astro',
    filename: 'index.astro',
    code: `---
import { getEntryBySlug } from 'astro:content';

const canonical = Astro.canonicalURL;
const posts = await Astro.fetchContent('../pages/post/*.md');
const active = true;
---

<h1>{canonical.pathname}</h1>
<img class={active ? 'active' : ''} src={Astro.resolve('../images/hero.png')} />
<p set:text={posts.length}></p>
`,
  },
  {
    label: 'Security',
    filename: 'app.js',
    code: `const cp = require('child_process');

function run(userInput) {
  cp.exec(userInput);
  eval(userInput);
  const pattern = new RegExp(userInput);
  return pattern.test('value');
}
`,
  },
  {
    label: 'Regular expressions',
    filename: 'patterns.js',
    code: `const a = /foo|foo/;
const b = /[0-9]/;
const c = /\\d{1,}/;
const d = new RegExp('(?:)');
`,
  },
  {
    label: 'TypeScript',
    filename: 'example.tsx',
    code: `import { useState } from 'react';

export function Counter() {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount(count + 1)}>{count}</button>;
}
`,
  },
  {
    label: 'JSON',
    filename: 'config.json',
    code: `{
  "name": "demo",
  "name": "duplicate-key",
  "values": [1, 2, 3,]
}
`,
  },
  {
    label: 'ESLint comments',
    filename: 'directives.js',
    code: `/* eslint-disable */
const cp = require('child_process');

// eslint-disable-next-line security/detect-child-process
cp.exec(userInput);
`,
  },
  {
    label: 'PostgreSQL',
    filename: 'schema.sql',
    code: `CREATE TABLE users (
  id int,
  Name text
);

SELECT * FROM users;
`,
  },
];
