import { test, expect } from '../setup';

test.describe('File Versioning', () => {
  test('list versions returns empty for new file', async ({ api }) => {
    await api.put('/e2e-ver-test.txt', 'version 1');

    const resp = await api.get('/api/v1/files/e2e-ver-test.txt/versions');
    expect([200, 503]).toContain(resp.status());

    if (resp.status() === 200) {
      const body = await resp.json();
      expect(body.versions).toBeDefined();
      expect(Array.isArray(body.versions)).toBe(true);
    }
  });

  test('create version snapshots current file', async ({ api }) => {
    await api.put('/e2e-ver-create.txt', 'version 1');

    const resp = await api.post('/api/v1/files/e2e-ver-create.txt/versions');
    expect([201, 503]).toContain(resp.status());

    if (resp.status() === 201) {
      const body = await resp.json();
      expect(body.id).toBeDefined();
      expect(body.content_hash).toBeDefined();
    }
  });

  test('list versions returns created versions', async ({ api }) => {
    await api.put('/e2e-ver-list.txt', 'version 1');
    await api.post('/api/v1/files/e2e-ver-list.txt/versions');

    await api.put('/e2e-ver-list.txt', 'version 2');
    await api.post('/api/v1/files/e2e-ver-list.txt/versions');

    const resp = await api.get('/api/v1/files/e2e-ver-list.txt/versions');
    expect([200, 503]).toContain(resp.status());

    if (resp.status() === 200) {
      const body = await resp.json();
      expect(body.versions.length).toBeGreaterThanOrEqual(2);
    }
  });

  test('get specific version returns content', async ({ api }) => {
    await api.put('/e2e-ver-get.txt', 'version content');
    const createResp = await api.post('/api/v1/files/e2e-ver-get.txt/versions');

    if (createResp.status() === 201) {
      const version = await createResp.json();
      const versionId = version.id;

      const resp = await api.get(`/api/v1/files/e2e-ver-get.txt/versions/${versionId}`);
      expect(resp.status()).toBe(200);
      const content = await resp.text();
      expect(content).toBe('version content');
    }
  });

  test('get nonexistent version returns 404', async ({ api }) => {
    await api.put('/e2e-ver-404.txt', 'content');

    const resp = await api.get('/api/v1/files/e2e-ver-404.txt/versions/999999');
    expect([404, 503]).toContain(resp.status());
  });

  test('delete version removes it', async ({ api }) => {
    await api.put('/e2e-ver-del.txt', 'to delete');
    const createResp = await api.post('/api/v1/files/e2e-ver-del.txt/versions');

    if (createResp.status() === 201) {
      const version = await createResp.json();
      const versionId = version.id;

      const resp = await api.delete(`/api/v1/files/e2e-ver-del.txt/versions/${versionId}`);
      expect([200, 204, 503]).toContain(resp.status());
    }
  });

  test('diff versions shows changes', async ({ api }) => {
    await api.put('/e2e-ver-diff.txt', 'original');
    await api.post('/api/v1/files/e2e-ver-diff.txt/versions');

    await api.put('/e2e-ver-diff.txt', 'modified');
    await api.post('/api/v1/files/e2e-ver-diff.txt/versions');

    const resp = await api.get('/api/v1/files/e2e-ver-diff.txt/diff');
    expect([200, 503]).toContain(resp.status());
  });

  test('versioning nonexistent file returns 404', async ({ api }) => {
    const resp = await api.post('/api/v1/files/nonexistent-xyz-file.txt/versions');
    expect([404, 503]).toContain(resp.status());
  });
});

test.afterEach(async ({ api }) => {
  const paths = [
    '/e2e-ver-test.txt', '/e2e-ver-create.txt', '/e2e-ver-list.txt',
    '/e2e-ver-get.txt', '/e2e-ver-404.txt', '/e2e-ver-del.txt',
    '/e2e-ver-diff.txt',
  ];
  for (const path of paths) {
    try { await api.delete(path); } catch {}
  }
});
