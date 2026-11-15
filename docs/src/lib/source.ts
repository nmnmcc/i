import { getCollection } from 'astro:content';
import { createGetUrl, llms, loader } from 'fumadocs-core/source';
import { toFumadocsSource } from '@fumadocs/astro/source';
import { docsRoute } from './shared';

export const source = loader({
  source: toFumadocsSource(await getCollection('docs'), await getCollection('meta')),
  baseUrl: docsRoute,
  url: (slugs, locale) => slugs.length ? createGetUrl(docsRoute)(slugs, locale) : '/',
  pageTree: { transformers: [{ file: (node) => node.url === '/' ? { ...node, name: 'Home' } : node }] },
});

export const docsLlms = llms(source, {
  renderPage: async (page) => `# ${page.data.title} (${page.url})

${await page.data.getText('processed')}`,
});
