/**
 * Ferro Load Test: Concurrent Upload Benchmark
 *
 * Requires: k6 (https://k6.io)
 * Usage:    k6 run concurrent-upload.js
 *
 * Measures throughput and error rate under 100+ simultaneous PUT requests.
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.2/index.js';

const BASE_URL = __ENV.FERRO_URL || 'http://localhost:8080';
const AUTH_HEADER = `Basic ${__ENV.FERRO_AUTH_B64 || 'YWRtaW46VGVzdFBhc3MxMjMh'}`;

export const options = {
  stages: [
    { duration: '10s', target: 20 },   // ramp up to 20 users
    { duration: '20s', target: 50 },   // ramp up to 50 users
    { duration: '30s', target: 100 },  // ramp up to 100 users
    { duration: '10s', target: 0 },    // ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.05'],
  },
};

export function setup() {
  // Create test directory
  const res = http.request('MKCOL', `${BASE_URL}/load-test-dir/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
  check(res, { 'setup MKCOL': (r) => [201, 405, 409].includes(r.status) });
}

export default function () {
  const fileId = __VU * 10000 + __ITER;
  const path = `/load-test-dir/file-${fileId}.txt`;
  const content = `load-test-content-${fileId}-${Date.now()}`;

  // PUT file
  const putRes = http.put(`${BASE_URL}${path}`, content, {
    headers: {
      Authorization: AUTH_HEADER,
      'Content-Type': 'text/plain',
    },
  });
  check(putRes, { 'PUT status 2xx': (r) => [200, 201, 204].includes(r.status) });

  // GET file to verify
  const getRes = http.get(`${BASE_URL}${path}`, {
    headers: { Authorization: AUTH_HEADER },
  });
  check(getRes, {
    'GET status 200': (r) => r.status === 200,
    'GET content matches': (r) => r.status === 200 && r.body === content,
  });

  // Cleanup
  http.del(`${BASE_URL}${path}`, null, {
    headers: { Authorization: AUTH_HEADER },
  });

  sleep(0.1);
}

export function teardown() {
  // Remove test directory
  http.del(`${BASE_URL}/load-test-dir/`, null, {
    headers: { Authorization: AUTH_HEADER },
  });
}

export function handleSummary(data) {
  const passed = data.metrics.http_req_failed
    ? data.metrics.http_req_failed.values.rate < 0.05
    : true;
  const p99 = data.metrics.http_req_duration
    ? data.metrics.http_req_duration.values['p(99)']
    : 0;

  return {
    stdout: textSummary(data, { indent: ' ', enableColors: true }),
    'load-test-results.json': JSON.stringify({
      passed,
      p99_ms: Math.round(p99),
      error_rate: data.metrics.http_req_failed?.values.rate || 0,
      iterations: data.metrics.iterations?.values.count || 0,
      rps: data.metrics.http_reqs?.values.rate || 0,
    }, null, 2),
  };
}
