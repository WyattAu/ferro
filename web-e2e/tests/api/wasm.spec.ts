import { test, expect } from '../setup';

test.describe('WASM Workers', () => {
  test('GET /api/v1/workers/modules returns list (empty or populated)', async ({ api }) => {
    const resp = await api.get('/api/v1/workers/modules');
    expect([200, 503]).toContain(resp.status());
  });

  test('GET /api/v1/workers returns list', async ({ api }) => {
    const resp = await api.get('/api/v1/workers');
    expect([200, 503]).toContain(resp.status());
  });

  test('POST /api/v1/workers/upload rejects non-WASM file', async ({ request, baseURL }) => {
    const base = new URL(baseURL!);
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');

    const resp = await request.post('/api/v1/workers/upload', {
      headers: { Authorization: authHeader },
      multipart: {
        file: {
          name: 'test.txt',
          mimeType: 'text/plain',
          buffer: Buffer.from('not wasm content'),
        },
      },
    });
    // Should reject: invalid WASM magic bytes
    expect([400, 503]).toContain(resp.status());
  });

  test('POST /api/v1/workers/upload rejects path traversal filename', async ({ request, baseURL }) => {
    const base = new URL(baseURL!);
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');

    // Valid WASM magic bytes + invalid filename with path traversal
    const wasmMagic = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);

    const resp = await request.post('/api/v1/workers/upload', {
      headers: { Authorization: authHeader },
      multipart: {
        file: {
          name: '../../../etc/evil.wasm',
          mimeType: 'application/wasm',
          buffer: wasmMagic,
        },
      },
    });
    expect([400, 503]).toContain(resp.status());
  });

  test('DELETE /api/v1/workers/modules/nonexistent returns 404 or 503', async ({ api }) => {
    const resp = await api.delete('/api/v1/workers/modules/nonexistent-module-xyz.wasm');
    expect([404, 503]).toContain(resp.status());
  });

  test('POST /api/v1/workers register and list', async ({ api }) => {
    const registerResp = await api.post('/api/v1/workers', {
      name: 'e2e-test-worker',
      trigger: 'on_upload',
      pattern: '*.txt',
      wasm_module: 'nonexistent.wasm',
    });
    // May succeed or fail if module doesn't exist
    expect([200, 201, 400, 404, 503]).toContain(registerResp.status());

    const listResp = await api.get('/api/v1/workers');
    expect([200, 503]).toContain(listResp.status());
  });
});

test.describe('WASM Metrics', () => {
  test('Prometheus metrics include WASM counters', async ({ api }) => {
    const resp = await api.get('/metrics');
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    // WASM counters should be present (may be 0)
    expect(body).toContain('ferro_wasm_dispatch_total');
    expect(body).toContain('ferro_wasm_errors_total');
    expect(body).toContain('ferro_wasm_fuel_consumed_total');
  });
});
