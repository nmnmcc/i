import { defineCollection } from 'astro:content';
import { metaLoader, pageLoader } from '@fumadocs/astro/loaders';

export const collections = {
  home: defineCollection({ loader: pageLoader('home') }),
  docs: defineCollection({ loader: pageLoader() }),
  meta: defineCollection({ loader: metaLoader() }),
};
