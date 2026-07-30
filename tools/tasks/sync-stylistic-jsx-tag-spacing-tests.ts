process.env.STYLISTIC_JSX_RULE = 'jsx-tag-spacing';

await import(new URL('./sync-stylistic-jsx-newline-tests.ts', import.meta.url).href);
