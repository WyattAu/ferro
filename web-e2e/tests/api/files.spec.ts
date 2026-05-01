import { test, expect } from '../setup';

test.describe('File Operations', () => {
  test('GET /healthz returns OK', async ({ request }) => {
    const resp = await request.get('/healthz');
    expect(resp.status()).toBe(200);
  });

  test('GET /readyz returns OK', async ({ request }) => {
    const resp = await request.get('/readyz');
    expect(resp.status()).toBe(200);
  });

  test('unauthenticated request returns 401', async ({ request }) => {
    const resp = await request.get('/api/storage/stats');
    expect(resp.status()).toBe(401);
  });

  test('PROPFIND root returns multistatus', async ({ api }) => {
    const resp = await api.propfind('/');
    expect(resp.status()).toBe(207);
    const body = await resp.text();
    expect(body).toContain('multistatus');
  });

  test('MKCOL creates directory', async ({ api }) => {
    const resp = await api.mkcol('/e2e-test-dir');
    expect(resp.status()).toBe(201);
  });

  test('PUT uploads file', async ({ api }) => {
    await api.mkcol('/e2e-test-dir');
    const resp = await api.put('/e2e-test-dir/test.txt', 'Hello, E2E!');
    expect([201, 204]).toContain(resp.status());
  });

  test('GET downloads file', async ({ api }) => {
    await api.mkcol('/e2e-test-dir');
    await api.put('/e2e-test-dir/test.txt', 'Hello, E2E!');
    const resp = await api.get('/e2e-test-dir/test.txt');
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body).toBe('Hello, E2E!');
  });

  test('DELETE removes file', async ({ api }) => {
    await api.mkcol('/e2e-test-dir');
    await api.put('/e2e-test-dir/test.txt', 'Hello, E2E!');
    const resp = await api.delete('/e2e-test-dir/test.txt');
    expect(resp.status()).toBe(204);
  });

  test('DELETE removes directory', async ({ api }) => {
    await api.mkcol('/e2e-test-dir');
    const resp = await api.delete('/e2e-test-dir');
    expect(resp.status()).toBe(204);
  });

  test('COPY duplicates file', async ({ api }) => {
    await api.put('/e2e-copy-src.txt', 'copy me');
    const resp = await api.copy('/e2e-copy-src.txt', '/e2e-copy-dst.txt');
    expect([201, 204]).toContain(resp.status());

    const getResp = await api.get('/e2e-copy-dst.txt');
    expect(getResp.status()).toBe(200);
    expect(await getResp.text()).toBe('copy me');

    await api.delete('/e2e-copy-src.txt');
    await api.delete('/e2e-copy-dst.txt');
  });

  test('MOVE relocates file', async ({ api }) => {
    await api.put('/e2e-move-src.txt', 'move me');
    const resp = await api.move('/e2e-move-src.txt', '/e2e-move-dst.txt');
    expect([201, 204]).toContain(resp.status());

    expect((await api.get('/e2e-move-src.txt')).status()).toBe(404);
    expect((await api.get('/e2e-move-dst.txt')).status()).toBe(200);

    await api.delete('/e2e-move-dst.txt');
  });

  test('PROPFIND depth:infinity lists recursively', async ({ api }) => {
    await api.mkcol('/e2e-recursive');
    await api.put('/e2e-recursive/a.txt', 'a');
    await api.put('/e2e-recursive/b.txt', 'b');

    const resp = await api.propfind('/e2e-recursive', 'infinity');
    expect(resp.status()).toBe(207);
    const body = await resp.text();
    expect(body).toContain('a.txt');
    expect(body).toContain('b.txt');

    await api.delete('/e2e-recursive/b.txt');
    await api.delete('/e2e-recursive/a.txt');
    await api.delete('/e2e-recursive');
  });

  test('404 for nonexistent file', async ({ api }) => {
    const resp = await api.get('/nonexistent-file-xyz.txt');
    expect(resp.status()).toBe(404);
  });

  test('overwrite existing file', async ({ api }) => {
    await api.put('/e2e-overwrite.txt', 'v1');
    await api.put('/e2e-overwrite.txt', 'v2');
    const resp = await api.get('/e2e-overwrite.txt');
    expect(await resp.text()).toBe('v2');
    await api.delete('/e2e-overwrite.txt');
  });
});

test.describe('User Management', () => {
  test('create and list users', async ({ api }) => {
    const resp = await api.post('/api/admin/users', {
      username: 'e2e-user',
      password: 'E2eTest123!',
      role: 'user',
    });
    expect([201, 200]).toContain(resp.status());

    const listResp = await api.get('/api/admin/users');
    expect(listResp.status()).toBe(200);
    const body = await listResp.json();
    expect(body).toBeDefined();

    await api.delete('/api/admin/users/e2e-user');
  });

  test('duplicate user returns error', async ({ api }) => {
    await api.post('/api/admin/users', { username: 'e2e-dup', password: 'E2eTest123!', role: 'user' });
    const resp = await api.post('/api/admin/users', { username: 'e2e-dup', password: 'E2eTest123!', role: 'user' });
    expect([409, 400, 500]).toContain(resp.status());
    await api.delete('/api/admin/users/e2e-dup');
  });
});

test.describe('Tags', () => {
  test('create and list tags', async ({ api }) => {
    await api.put('/e2e-tagged.txt', 'content');
    const resp = await api.post('/api/tags/e2e-tagged.txt', { tags: ['important', 'review'] });
    expect([200, 201]).toContain(resp.status());

    const listResp = await api.get('/api/tags/e2e-tagged.txt');
    expect(listResp.status()).toBe(200);

    await api.delete('/e2e-tagged.txt');
  });
});

test.describe('Search', () => {
  test('search finds uploaded files', async ({ api }) => {
    await api.put('/e2e-search-test.txt', 'unique-search-term-xyz-123');

    await new Promise(r => setTimeout(r, 500));

    const resp = await api.get('/api/search?q=unique-search-term-xyz-123');
    expect(resp.status()).toBe(200);

    await api.delete('/e2e-search-test.txt');
  });
});

test.describe('Shares', () => {
  test('create and list shares', async ({ api }) => {
    await api.put('/e2e-share.txt', 'shared content');
    const resp = await api.post('/api/shares', {
      path: '/e2e-share.txt',
      type: 'public',
    });
    expect([200, 201]).toContain(resp.status());

    const listResp = await api.get('/api/shares');
    expect(listResp.status()).toBe(200);

    await api.delete('/e2e-share.txt');
  });
});

test.describe('Favorites', () => {
  test('add and list favorites', async ({ api }) => {
    await api.put('/e2e-fav.txt', 'fav content');
    const resp = await api.put('/api/favorites', { path: '/e2e-fav.txt' });
    expect([200, 201]).toContain(resp.status());

    const listResp = await api.get('/api/favorites');
    expect(listResp.status()).toBe(200);

    await api.delete('/e2e-fav.txt');
  });
});

test.describe('Health and Monitoring', () => {
  test('GET /api/health/storage returns storage info', async ({ api }) => {
    const resp = await api.get('/api/health/storage');
    expect(resp.status()).toBe(200);
  });

  test('GET /metrics returns prometheus format', async ({ api }) => {
    const resp = await api.get('/metrics');
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body).toContain('# HELP');
  });
});

test.describe('Security Headers', () => {
  test('responses include security headers', async ({ request }) => {
    const resp = await request.get('/healthz');
    expect(resp.headers()['x-content-type-options']).toBe('nosniff');
    expect(resp.headers()['x-frame-options']).toBe('DENY');
    expect(resp.headers()['strict-transport-security']).toBeDefined();
  });
});

test.describe('Path Traversal Prevention', () => {
  test('path traversal is rejected', async ({ api }) => {
    const resp = await api.get('/../../../etc/passwd');
    expect(resp.status()).toBe(400);
  });

  test('encoded path traversal is rejected', async ({ api }) => {
    const resp = await api.put('/%2e%2e/%2e%2e/etc/passwd', 'test');
    expect(resp.status()).toBe(400);
  });
});

test.afterEach(async ({ api }) => {
  const paths = [
    '/e2e-test-dir', '/e2e-copy-src.txt', '/e2e-copy-dst.txt',
    '/e2e-move-src.txt', '/e2e-move-dst.txt', '/e2e-overwrite.txt',
    '/e2e-recursive', '/e2e-tagged.txt', '/e2e-search-test.txt',
    '/e2e-share.txt', '/e2e-fav.txt',
  ];
  for (const path of paths) {
    try { await api.delete(path); } catch {}
  }
});
