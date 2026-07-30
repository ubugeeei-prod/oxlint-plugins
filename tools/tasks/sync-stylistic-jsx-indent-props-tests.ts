process.env.STYLISTIC_JSX_RULE = 'jsx-indent-props';

await import(new URL('./sync-stylistic-jsx-newline-tests.ts', import.meta.url).href);
