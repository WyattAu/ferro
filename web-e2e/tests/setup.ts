import { test as base, expect } from '@playwright/test';

const AUTH_USER = 'e2e-admin';
const AUTH_PASS = 'e2e-test-token';

function basicAuthHeader(): string {
  return 'Basic ' + Buffer.from(`${AUTH_USER}:${AUTH_PASS}`).toString('base64');
}

const test = base.extend({
  api: async ({ request, baseURL }, use) => {
    const base = new URL(baseURL!);
    const authHeader = basicAuthHeader();

    const api = {
      get: (path: string) =>
        request.get(path, { headers: { Authorization: authHeader } }),

      post: (path: string, data?: any) =>
        request.post(path, {
          headers: { Authorization: authHeader, 'Content-Type': 'application/json' },
          data,
        }),

      put: (path: string, data?: any) =>
        request.put(path, {
          headers: { Authorization: authHeader, 'Content-Type': 'application/octet-stream' },
          data,
        }),

      delete: (path: string) =>
        request.delete(path, { headers: { Authorization: authHeader } }),

      propfind: (path: string, depth: string = '1') =>
        request.fetch(path, {
          method: 'PROPFIND',
          headers: {
            Authorization: authHeader,
            Depth: depth,
            'Content-Type': 'application/xml',
          },
          body: `<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
  </D:prop>
</D:propfind>`,
        }),

      mkcol: (path: string) =>
        request.fetch(path, {
          method: 'MKCOL',
          headers: { Authorization: authHeader },
        }),

      copy: (path: string, destination: string) =>
        request.fetch(path, {
          method: 'COPY',
          headers: {
            Authorization: authHeader,
            Destination: destination,
          },
        }),

      move: (path: string, destination: string) =>
        request.fetch(path, {
          method: 'MOVE',
          headers: {
            Authorization: authHeader,
            Destination: destination,
          },
        }),
    };

    await use(api);
  },
});

export { test, expect };
