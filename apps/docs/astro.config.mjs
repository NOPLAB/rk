import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://noplab.github.io',
  base: '/rk',
  integrations: [
    starlight({
      title: 'RK',
      description: 'Documentation for RK',
      logo: {
        src: './src/assets/logo.svg',
        replacesTitle: true,
      },
      favicon: '/favicon.svg',
      social: {
        github: 'https://github.com/NOPLAB/rk',
      },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Quick Start', slug: 'getting-started/quick-start' },
          ],
        },
        {
          label: 'Guides',
          autogenerate: { directory: 'guides' },
        },
        {
          label: 'Reference',
          autogenerate: { directory: 'reference' },
        },
      ],
      editLink: {
        baseUrl: 'https://github.com/NOPLAB/rk/edit/main/apps/docs/',
      },
    }),
  ],
});
