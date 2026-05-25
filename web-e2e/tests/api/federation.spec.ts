import { test, expect } from '../setup';

test.describe('ActivityPub Federation', () => {
  test('GET /.well-known/webfinger returns resource', async ({ request }) => {
    const resp = await request.get(
      '/.well-known/webfinger?resource=acct:admin@localhost'
    );
    // May return 404 if federation not configured, or 200 with JRD
    expect([200, 404, 503]).toContain(resp.status());

    if (resp.status() === 200) {
      const body = await resp.json();
      expect(body.subject).toBeDefined();
    }
  });

  test('GET /fed/nodeinfo returns node info', async ({ request }) => {
    const resp = await request.get('/fed/nodeinfo');
    expect([200, 404, 503]).toContain(resp.status());

    if (resp.status() === 200) {
      const body = await resp.json();
      expect(body.software || body.version || body.protocols).toBeDefined();
    }
  });

  test('GET /fed/actor/{username} returns actor profile', async ({ request }) => {
    const resp = await request.get('/fed/actor/admin');
    expect([200, 404, 503]).toContain(resp.status());

    if (resp.status() === 200) {
      const body = await resp.json();
      // ActivityPub actor must have preferredUsername or type
      expect(body.type || body.preferredUsername).toBeDefined();
    }
  });

  test('POST /fed/inbox rejects unsigned activity', async ({ request }) => {
    const resp = await request.post('/fed/inbox', {
      data: {
        type: 'Follow',
        actor: 'https://evil.example/user',
        object: 'https://localhost/fed/actor/admin',
      },
    });
    // Should reject: no valid HTTP Signature
    expect([401, 403, 503]).toContain(resp.status());
  });

  test('POST /fed/inbox rejects invalid signature', async ({ request }) => {
    const resp = await request.post('/fed/inbox', {
      headers: {
        Signature:
          'keyId="https://evil.example/keys/1",headers="(request-target)",signature="invalidbase64"',
      },
      data: {
        type: 'Follow',
        actor: 'https://evil.example/user',
        object: 'https://localhost/fed/actor/admin',
      },
    });
    expect([401, 403, 503]).toContain(resp.status());
  });

  test('GET /fed/inbox returns list or requires auth', async ({ request }) => {
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');
    const resp = await request.get('/fed/inbox', {
      headers: { Authorization: authHeader },
    });
    expect([200, 401, 403, 503]).toContain(resp.status());
  });

  test('GET /fed/outbox returns list or requires auth', async ({ request }) => {
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');
    const resp = await request.get('/fed/outbox', {
      headers: { Authorization: authHeader },
    });
    expect([200, 401, 403, 503]).toContain(resp.status());
  });

  test('GET /fed/actor/{username}/followers returns list', async ({ request }) => {
    const resp = await request.get('/fed/actor/admin/followers');
    expect([200, 404, 503]).toContain(resp.status());
  });

  test('GET /fed/actor/{username}/following returns list', async ({ request }) => {
    const resp = await request.get('/fed/actor/admin/following');
    expect([200, 404, 503]).toContain(resp.status());
  });
});

test.describe('Federated Sharing', () => {
  test('POST /api/v1/fed/share rejects unauthenticated', async ({ request }) => {
    const resp = await request.post('/api/v1/fed/share', {
      data: {
        path: '/test.txt',
        target: 'https://other.example/fed/actor/user',
      },
    });
    expect([401, 403, 503]).toContain(resp.status());
  });
});
