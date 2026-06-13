const { execSync, spawn } = require('child_process');
const http = require('http');
const fs = require('fs');

const PORT = 8080;
const BASE_URL = `http://127.0.0.1:${PORT}`;

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

function curl(method, path, body = null) {
  return new Promise((resolve, reject) => {
    const url = `${BASE_URL}${path}`;
    const opts = { method, timeout: 5000 };
    if (body) opts.body = body;
    
    const req = http.request(url, opts, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => resolve({ status: res.statusCode, body: data }));
    });
    req.on('error', reject);
    req.setTimeout(5000, () => { req.destroy(); reject(new Error('timeout')); });
    if (body) req.write(body);
    req.end();
  });
}

async function main() {
  console.log('=== Ferro Parallel Deployment Test ===');
  console.log(`Start: ${new Date().toISOString()}`);
  
  // Start server
  console.log('\nStarting Ferro server...');
  const server = spawn('target/debug/ferro-server', 
    ['--host', '127.0.0.1', '--port', String(PORT), '--static-dir', 'crates/web/dist'],
    { stdio: 'ignore', detached: true }
  );
  server.unref();
  
  // Wait for server
  for (let i = 0; i < 30; i++) {
    await sleep(1000);
    try {
      const r = await curl('GET', '/.well-known/ferro');
      if (r.status === 200) {
        console.log(`Server ready after ${i+1}s`);
        break;
      }
    } catch (e) {}
  }
  
  let pass = 0, fail = 0, total = 0;
  
  async function check(name, expected, fn) {
    total++;
    try {
      const result = await fn();
      if (result === expected) {
        console.log(`  PASS: ${name}`);
        pass++;
      } else {
        console.log(`  FAIL: ${name} (expected=${expected} got=${result})`);
        fail++;
      }
    } catch (e) {
      console.log(`  FAIL: ${name} (${e.message})`);
      fail++;
    }
  }
  
  console.log('\n=== HEALTH ===');
  await check('health', 200, async () => (await curl('GET', '/.well-known/ferro')).status);
  await check('healthz', 200, async () => (await curl('GET', '/healthz')).status);
  
  console.log('\n=== STATIC ===');
  await check('root', 200, async () => (await curl('GET', '/')).status);
  await check('css', 200, async () => (await curl('GET', '/ui/style.css')).status);
  
  console.log('\n=== WEBDAV ===');
  await check('MKCOL', 201, async () => (await curl('MKCOL', '/test')).status);
  await check('PUT', 201, async () => (await curl('PUT', '/test/hello.txt', 'hello')).status);
  await check('GET', 'hello', async () => (await curl('GET', '/test/hello.txt')).body);
  await check('MOVE', 201, async () => {
    const r = await new Promise((resolve, reject) => {
      const req = http.request(`${BASE_URL}/test/hello.txt`, {
        method: 'MOVE',
        headers: { 'Destination': '/test/renamed.txt' }
      }, res => resolve({ status: res.statusCode }));
      req.on('error', reject);
      req.end();
    });
    return r.status;
  });
  await check('DELETE', 204, async () => {
    const r = await new Promise((resolve, reject) => {
      const req = http.request(`${BASE_URL}/test/renamed.txt`, { method: 'DELETE' }, 
        res => resolve({ status: res.statusCode }));
      req.on('error', reject);
      req.end();
    });
    return r.status;
  });
  await check('DELETE-dir', 204, async () => {
    const r = await new Promise((resolve, reject) => {
      const req = http.request(`${BASE_URL}/test`, { method: 'DELETE' }, 
        res => resolve({ status: res.statusCode }));
      req.on('error', reject);
      req.end();
    });
    return r.status;
  });
  
  console.log('\n=== API ===');
  await check('config', 200, async () => (await curl('GET', '/api/config')).status);
  await check('search', 200, async () => (await curl('GET', '/api/search?q=test')).status);
  
  console.log('\n=== SECURITY ===');
  await check('no-shell', 404, async () => (await curl('GET', '/bin/sh')).status);
  
  console.log(`\n================================`);
  console.log(`RESULTS: ${pass} PASS / ${fail} FAIL / ${total} TOTAL`);
  console.log(`================================`);
  
  // Kill server
  process.kill(-server.pid, 'SIGTERM');
  console.log(`\nEnd: ${new Date().toISOString()}`);
}

main().catch(console.error);
