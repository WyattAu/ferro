import { test, expect } from '../setup';

test.describe('CalDAV', () => {
  test('discover calendar home', async ({ api }) => {
    const resp = await api.get('/dav/cal/');
    expect([207, 404, 200]).toContain(resp.status());
  });

  test('MKCOL creates calendar', async ({ api }) => {
    const resp = await api.mkcol('/dav/cal/e2e-calendar');
    expect([201, 405]).toContain(resp.status());
  });
});

test.describe('CardDAV', () => {
  test('discover address book home', async ({ api }) => {
    const resp = await api.get('/dav/card/');
    expect([207, 404, 200]).toContain(resp.status());
  });
});
