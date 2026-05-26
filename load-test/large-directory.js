/**
 * Ferro Load Test: Large Directory Listing
 *
 * Requires: k6 (https://k6.io)
 * Usage:    k6 run large-directory.js
 *
 * Creates 10,000+ files then benchmarks PROPFIND listing performance.
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.2/index.js';

const BASE_URL = __ENV.FERRO_URL || 'http://localhost:8080';
const AUTH_HEADER = `Basic ${__ENV.FERRO_AUTH_B64 || 'YWRtaW46VGVzdFBhc3MxMjMh'}`;
const FILE_COUNT = parseInt(__ENV.FILE_COUNT || '1000', 10);

export const options = {
  scenarios: {
    populate: {
      executor: 'shared-iterations',
      iterations: FILE_COUNT,
      vus: 20,
      exec: 'populate',
    },
    list: {
      executor: 'per-vu-iterations',
      vus: 10,
      iterations: 5,
      exec: 'listFiles',
      startTime: '30s',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<2000', 'p(99)<5000'],
    http_req_failed: ['rate<0.01'],
    'http_req_duration{scenario:list}': ['p(95)<500'],
  },
};

export function setup() {
  const res = http.request('MKCOL', `${BASE_URL}/load-test-list/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
  check(res, { 'setup MKCOL': (r) => [201, 405, 409].includes(r.status) });
}

export function populate() {
  const fileId = __VU * 100000 + __ITER;
  const path = `/load-test-list/file-${fileId}.txt`;
  const content = `file-content-${fileId}`;

  const res = http.put(`${BASE_URL}${path}`, content, {
    headers: {
      Authorization: AUTH_HEADER,
      'Content-Type': 'text/plain',
    },
  });
  check(res, { 'PUT ok': (r) => [200, 201, 204].includes(r.status) });
}

export function listFiles() {
  const propfindBody = `<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
  </D:prop>
</D:propfind>`;

  // Depth 1 listing
  const res1 = http.request('PROPFIND', `${BASE_URL}/load-test-list/`, propfindBody, {
    headers: {
      Authorization: AUTH_HEADER,
      Depth: '1',
      'Content-Type': 'application/xml',
    },
  });
  check(res1, {
    'PROPFIND depth:1 status 207': (r) => r.status === 207,
    'PROPFIND depth:1 has multistatus': (r) => r.body && r.body.includes('multistatus'),
  });

  sleep(0.5);

  // Depth infinity listing
  const res2 = http.request('PROPFIND', `${BASE_URL}/load-test-list/`, propfindBody, {
    headers: {
      Authorization: AUTH_HEADER,
      Depth: 'infinity',
      'Content-Type': 'application/xml',
    },
  });
  check(res2, {
    'PROPFIND infinity status 207 or 403': (r) => [207, 403].includes(r.status),
  });
}

export function teardown() {
  // Best-effort cleanup
  http.del(`${BASE_URL}/load-test-list/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
}

export function handleSummary(data) {
  return {
    stdout: textSummary(data, { indent: ' ', enableColors: true }),
    'load-test-list-results.json': JSON.stringify({
      file_count: FILE_COUNT,
      list_p99_ms: data.metrics['http_req_duration{scenario:list}']
        ? Math.round(data.metrics['http_req_duration{scenario:list}'].values['p(99)'])
        : null,
      error_rate: data.metrics.http_req_failed?.values.rate || 0,
    }, null, 2),
  };
}
