import { defineCollection } from 'astro:content';
import { z } from 'astro/zod';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

export const collections = {
  // A page without a description has no lead, no search snippet and no llms.txt line.
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({ extend: z.object({ description: z.string().min(1) }) }),
  }),
};
