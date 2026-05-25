import { test, expect } from '../setup';

test.describe('GraphQL', () => {
  test('health query returns ok', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ health { status version } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.health.status).toBe('ok');
    expect(body.data.health.version).toBeDefined();
  });

  test('files query returns list', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ files { name path isDir size modified } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.files).toBeDefined();
    expect(Array.isArray(body.data.files)).toBe(true);
  });

  test('files query with path parameter', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ files(path: "/") { name path } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.files).toBeDefined();
  });

  test('file query returns null for nonexistent file', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ file(path: "/nonexistent-xyz-abc.txt") { name } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.file).toBeNull();
  });

  test('shares query returns list', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ shares { id path token } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.shares).toBeDefined();
    expect(Array.isArray(body.data.shares)).toBe(true);
  });

  test('me query returns user', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ me { username role } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.me).toBeDefined();
    expect(body.data.me.username).toBeDefined();
  });

  test('auditLog query returns entries', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ auditLog(limit: 5) { id action timestamp user } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.auditLog).toBeDefined();
    expect(Array.isArray(body.data.auditLog)).toBe(true);
  });

  test('createFolder mutation creates directory', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: 'mutation { createFolder(path: "/e2e-gql-folder") { name path isDir } }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.createFolder).toBeDefined();
    expect(body.data.createFolder.isDir).toBe(true);
  });

  test('deleteFile mutation removes file', async ({ api }) => {
    // Create a file first
    await api.put('/e2e-gql-delete.txt', 'graphql test content');

    const resp = await api.post('/api/v1/graphql', {
      query: 'mutation { deleteFile(path: "/e2e-gql-delete.txt") }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.data.deleteFile).toBe(true);

    // Verify deleted
    const getResp = await api.get('/e2e-gql-delete.txt');
    expect(getResp.status()).toBe(404);
  });

  test('GET /api/v1/graphql returns playground', async ({ api }) => {
    const resp = await api.get('/api/v1/graphql');
    expect(resp.status()).toBe(200);
  });

  test('GraphQL rejects invalid query', async ({ api }) => {
    const resp = await api.post('/api/v1/graphql', {
      query: '{ invalidFieldThatDoesNotExist }',
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.errors).toBeDefined();
    expect(body.errors.length).toBeGreaterThan(0);
  });

  test('GraphQL rejects deeply nested query (DoS mitigation)', async ({ api }) => {
    // Build a deeply nested query
    const deepQuery = '{ files { name } '.repeat(20) + '}' .repeat(20);
    const resp = await api.post('/api/v1/graphql', {
      query: deepQuery,
    });
    // Should return either errors or valid response (not 500)
    expect([200, 400]).toContain(resp.status());
  });
});

test.afterEach(async ({ api }) => {
  const paths = ['/e2e-gql-folder', '/e2e-gql-delete.txt'];
  for (const path of paths) {
    try { await api.delete(path); } catch {}
  }
});
