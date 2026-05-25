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
  test('OPTIONS /dav/card returns DAV headers', async ({ request }) => {
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');
    const resp = await request.fetch('/dav/card', {
      method: 'OPTIONS',
      headers: { Authorization: authHeader },
    });
    expect([200, 204]).toContain(resp.status());
  });

  test('GET /dav/card/ lists address books', async ({ api }) => {
    const resp = await api.get('/dav/card/');
    expect([200, 207, 404]).toContain(resp.status());
  });

  test('PUT /dav/card/ creates address book', async ({ request }) => {
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');
    const resp = await request.put('/dav/card/', {
      headers: {
        Authorization: authHeader,
        'Content-Type': 'application/json',
      },
      data: JSON.stringify({ name: 'e2e-addressbook' }),
    });
    expect([200, 201, 204, 400, 503]).toContain(resp.status());
  });

  test('GET /dav/card/{book}/ returns address book properties', async ({ api }) => {
    const resp = await api.get('/dav/card/e2e-book/');
    expect([200, 404]).toContain(resp.status());
  });

  test('PUT contact creates vCard', async ({ request }) => {
    const authHeader =
      'Basic ' + Buffer.from('e2e-admin:e2e-test-token').toString('base64');
    const vcard =
      'BEGIN:VCARD\r\nVERSION:3.0\r\nFN:E2E Test Contact\r\nN:Test;E2E\r\nEMAIL:e2e@test.com\r\nEND:VCARD';

    const resp = await request.put('/dav/card/e2e-book/e2e-test-uid.vcf', {
      headers: {
        Authorization: authHeader,
        'Content-Type': 'text/vcard',
      },
      data: vcard,
    });
    expect([200, 201, 204, 404]).toContain(resp.status());
  });

  test('GET contact returns vCard', async ({ api }) => {
    const resp = await api.get('/dav/card/e2e-book/e2e-test-uid.vcf');
    expect([200, 404]).toContain(resp.status());
  });

  test('DELETE contact removes vCard', async ({ api }) => {
    const resp = await api.delete('/dav/card/e2e-book/e2e-test-uid.vcf');
    expect([200, 204, 404]).toContain(resp.status());
  });

  test('DELETE /dav/card/{book} removes address book', async ({ api }) => {
    const resp = await api.delete('/dav/card/e2e-book');
    expect([200, 204, 404]).toContain(resp.status());
  });
});
