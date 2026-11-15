// @ts-check
import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import fumadocs from '@fumadocs/astro';
import { readFileSync } from 'node:fs';
import { fenceMetadata } from './scripts/fence-metadata.mjs';

const grammar = JSON.parse(readFileSync(new URL('../editors/vscode/syntaxes/i.tmLanguage.json', import.meta.url), 'utf8'));
grammar.name = 'i';

export default defineConfig({
  site: 'https://i.nmnm.cc',
  base: '/',
  integrations: [
    fumadocs({
      collections: {
        docs: { dir: 'content/docs' },
        home: { dir: 'content/home' },
      },
      components: './src/components/mdx.tsx',
      mdastPlugins: [fenceMetadata],
      rehypeCodeOptions: { langs: [grammar], fallbackLanguage: 'text', engine: 'oniguruma' },
      includeProcessedMarkdown: true,
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
});
