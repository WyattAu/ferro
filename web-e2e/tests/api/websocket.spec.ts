import { test, expect } from '@playwright/test';

test.describe('WebSocket', () => {
  test('connects with valid credentials', async ({ request }) => {
    const baseURL = process.env.FERRO_URL || 'http://localhost:8080';
    const wsUrl = baseURL.replace('http', 'ws') + '/api/ws';
    const auth = 'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');

    let connected = false;

    const ws = new WebSocket(wsUrl, {
      headers: { Authorization: auth },
    });
    ws.onopen = () => { connected = true; };
    ws.onerror = () => {};

    await new Promise(resolve => setTimeout(resolve, 2000));
    ws.close();

    expect(connected).toBe(true);
  });

  test('rejects invalid credentials', async () => {
    const baseURL = process.env.FERRO_URL || 'http://localhost:8080';
    const wsUrl = baseURL.replace('http', 'ws') + '/api/ws';
    const auth = 'Basic ' + Buffer.from('wrong:wrong').toString('base64');

    let connected = false;

    const ws = new WebSocket(wsUrl, {
      headers: { Authorization: auth },
    });
    ws.onopen = () => { connected = true; };
    ws.onerror = () => {};

    await new Promise(resolve => setTimeout(resolve, 2000));
    ws.close();

    expect(connected).toBe(false);
  });
});
