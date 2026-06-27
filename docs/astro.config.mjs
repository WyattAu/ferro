// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
  site: 'https://wyattau.github.io',
  base: '/ferro',
  integrations: [
    starlight({
      title: 'Ferro Documentation',
      logo: {
        src: './src/assets/logo.svg',
        alt: 'Ferro',
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/WyattAu/ferro' },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'introduction' },
            { label: 'Quick Start', slug: 'quickstart' },
            { label: 'Installation', slug: 'installation' },
            { label: 'Configuration', slug: 'configuration' },
          ],
        },
        {
          label: 'Architecture',
          items: [
            { label: 'Overview', slug: 'architecture' },
            { label: 'Architecture Deep Dive', slug: 'architecture-overview' },
          ],
        },
        {
          label: 'API Reference',
          items: [
            { label: 'Overview', slug: 'api-reference' },
            { label: 'WebDAV', slug: 'api/webdav' },
            { label: 'REST API', slug: 'api/rest' },
            { label: 'Admin API', slug: 'api/admin' },
            { label: 'CalDAV', slug: 'api/caldav' },
            { label: 'CardDAV', slug: 'api/carddav' },
            { label: 'GraphQL', slug: 'api/graphql' },
            { label: 'WebSocket', slug: 'api/websocket' },
            { label: 'Federation', slug: 'api/federation' },
            { label: 'Chunked Upload', slug: 'api/chunked-upload' },
          ],
        },
        {
          label: 'Library Crates',
          items: [
            { label: 'ferro-common', slug: 'crates/common' },
            { label: 'ferro-core', slug: 'crates/core' },
            { label: 'ferro-dav', slug: 'crates/dav' },
            { label: 'ferro-crypto', slug: 'crates/crypto' },
            { label: 'ferro-client', slug: 'crates/client' },
            { label: 'ferro-fuse', slug: 'crates/fuse' },
          ],
        },
        {
          label: 'Deployment',
          items: [
            { label: 'Docker', slug: 'deployment/docker' },
            { label: 'Kubernetes', slug: 'deployment/kubernetes' },
            { label: 'Podman', slug: 'deployment/podman' },
            { label: 'Firecracker', slug: 'deployment/firecracker' },
            { label: 'Terraform', slug: 'deployment/terraform' },
            { label: 'Blue-Green', slug: 'deployment/blue-green' },
            { label: 'Horizontal Scaling', slug: 'deployment/horizontal-scaling' },
            { label: 'PostgreSQL Migration', slug: 'deployment/postgresql-migration' },
            { label: 'Production Guide', slug: 'deployment/production' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Desktop App', slug: 'guides/desktop-app' },
            { label: 'FUSE Mount', slug: 'guides/fuse-mount' },
            { label: 'CalDAV Clients', slug: 'guides/caldav-clients' },
            { label: 'Encryption', slug: 'guides/encryption' },
            { label: 'Federation Setup', slug: 'guides/federation' },
            { label: 'Office Suite', slug: 'guides/office-suite' },
            { label: 'Platform Integration', slug: 'guides/platform-integration' },
            { label: 'Upgrade Guide', slug: 'guides/upgrade' },
          ],
        },
        {
          label: 'Security',
          items: [
            { label: 'Overview', slug: 'security' },
            { label: 'Compliance', slug: 'security/compliance' },
          ],
        },
      ],
    }),
  ],
});
