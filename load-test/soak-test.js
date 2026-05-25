/**
 * Ferro Load Test: Soak Test
 *
 * Requires: k6 (https://k6.io)
 * Usage:    k6 run soak-test.js --duration 1h
 *
 * Continuous random operations for stability testing.
 * Designed to run for extended periods (1-24 hours).
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { randomIntBetween, randomString } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

const BASE_URL = __ENV.FERRO_URL || 'http://localhost:8080';
const AUTH = __ENV.FERRO_AUTH || 'admin:TestPass123!';
const AUTH_HEADER = `Basic ${__ENCODING.b64encode(AUTH)}`;

export const options = {
  vus: 10,
  duration: '1h',
  thresholds: {
    http_req_duration: ['p(95)<1000'],
    http_req_failed: ['rate<0.02'],
  },
};

export function setup() {
  const res = http.request('MKCOL', `${BASE_URL}/soak-test/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
  check(res, { 'setup MKCOL': (r) => [201, 405, 409].includes(r.status) });
}

export default function () {
  const vuId = __VU;
  const iterId = __ITER;
  const path = `/soak-test/vu${vuId}-iter${iterId}.txt`;

  const op = randomIntBetween(0, 5);

  switch (op) {
    case 0: {
      // PUT (create/overwrite)
      const content = `soak-${randomString(32)}`;
      const res = http.put(`${BASE_URL}${path}`, content, {
        headers: { Authorization: AUTH_HEADER, 'Content-Type': 'text/plain' },
      });
      check(res, { 'PUT ok': (r) => [200, 201, 204].includes(r.status) });
      break;
    }
    case 1: {
      // GET
      const res = http.get(`${BASE_URL}${path}`, {
        headers: { Authorization: AUTH_HEADER },
      });
      check(res, { 'GET status valid': (r) => [200, 404].includes(r.status) });
      break;
    }
    case 2: {
      // DELETE
      const res = http.del(`${BASE_URL}${path}`, null, {
        headers: { Authorization: AUTH_HEADER },
      });
      check(res, { 'DELETE status valid': (r) => [200, 204, 404].includes(r.status) });
      break;
    }
    case 3: {
      // PROPFIND
      const res = http.request('PROPFIND', `${BASE_URL}/soak-test/`, `<?xml version="1.0"?>
<D:propfind xmlns:D="DAV:"><D:prop><D:resourcetype/></D:prop></D:propfind>`, {
        headers: {
          Authorization: AUTH_HEADER,
          Depth: '1',
          'Content-Type': 'application/xml',
        },
      });
      check(res, { 'PROPFIND 207': (r) => r.status === 207 });
      break;
    }
    case 4: {
      // COPY
      const srcPath = `/soak-test/vu${vuId}-iter${Math.max(0, iterId - 1)}.txt`;
      const dstPath = `/soak-test/vu${vuId}-copy-${iterId}.txt`;
      const res = http.request('COPY', `${BASE_URL}${srcPath}`, null, {
        headers: { Authorization: AUTH_HEADER, Destination: dstPath },
      });
      check(res, { 'COPY status valid': (r) => [201, 204, 404].includes(r.status) });
      break;
    }
    case 5: {
      // Health check
      const res = http.get(`${BASE_URL}/healthz`);
      check(res, { 'healthz 200': (r) => r.status === 200 });
      break;
    }
  }

  sleep(randomIntBetween(1, 5));
}

export function teardown() {
  http.del(`${BASE_URL}/soak-test/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
}
