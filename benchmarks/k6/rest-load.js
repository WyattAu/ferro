import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const BASE_URL = __ENV.FERRO_URL || 'http://localhost:9999';
const AUTH_HEADER = { 'Authorization': 'Basic ' + (__ENV.FERRO_AUTH || 'YWRtaW46dGVzdHBhc3MxMjM=') };
const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '30s', target: 10 },
    { duration: '1m', target: 50 },
    { duration: '30s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    errors: ['rate<0.01'],
  },
};

export default function () {
  // Health check (no auth)
  const healthRes = http.get(`${BASE_URL}/healthz`);
  check(healthRes, { 'health 200': (r) => r.status === 200 });

  // List files (auth required)
  const listRes = http.get(`${BASE_URL}/api/v1/files`, { headers: AUTH_HEADER });
  check(listRes, { 'list 200': (r) => r.status === 200 });
  errorRate.add(listRes.status >= 400);

  sleep(0.1);
}
