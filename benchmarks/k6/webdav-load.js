import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const BASE_URL = __ENV.FERRO_URL || 'http://localhost:9999';
const AUTH = __ENV.FERRO_AUTH || 'YWRtaW46dGVzdHBhc3MxMjM=';

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
  const authHeaders = { 'Authorization': 'Basic ' + AUTH, 'Content-Type': 'text/plain' };

  // PUT
  const putRes = http.put(
    `${BASE_URL}/remote.php/dav/files/admin/loadtest-${__VU}-${__ITER}.txt`,
    `load-test-content-${__ITER}`,
    { headers: authHeaders }
  );
  check(putRes, { 'PUT 201': (r) => r.status === 201 || r.status === 204 });

  // GET
  const getRes = http.get(
    `${BASE_URL}/remote.php/dav/files/admin/loadtest-${__VU}-${__ITER}.txt`,
    { headers: { 'Authorization': 'Basic ' + AUTH } }
  );
  check(getRes, { 'GET 200': (r) => r.status === 200 });

  // PROPFIND
  const propfindRes = http.request(
    'PROPFIND',
    `${BASE_URL}/remote.php/dav/files/admin/`,
    null,
    { headers: { 'Authorization': 'Basic ' + AUTH, 'Depth': '1' } }
  );
  check(propfindRes, { 'PROPFIND 207': (r) => r.status === 207 });

  sleep(0.05);
}
