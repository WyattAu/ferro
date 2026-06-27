//! Sustained load (soak) test harness for ferro-server.
//!
//! Spawns a real ferro-server process and hammers it with a mixed WebDAV
//! workload for a configurable duration. Tracks latency, error rates, and
//! server memory usage, then writes results to `target/soak-results.json`.
//!
//! Ignored by default — run with:
//!   cargo test -p ferro-server --test soak_test -- --ignored
//!
//! Or use `scripts/soak-test.sh` for a one-command runner.

use std::collections::HashMap;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use reqwest::Method;
use serde::Serialize;
use tokio::time::sleep;

// ── Tunables (env overrides) ─────────────────────────────────────────

fn soak_duration() -> Duration {
    let secs = std::env::var("SOAK_DURATION")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);
    Duration::from_secs(secs)
}

fn concurrent_users() -> usize {
    std::env::var("SOAK_USERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50)
}

// ── Operation distribution ───────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Op {
    Put,
    Get,
    Propfind,
    Delete,
    MoveCopy,
}

fn pick_op(rng: &mut impl Rng) -> Op {
    let r: f64 = rng.random();
    if r < 0.40 {
        Op::Put
    } else if r < 0.70 {
        Op::Get
    } else if r < 0.85 {
        Op::Propfind
    } else if r < 0.95 {
        Op::Delete
    } else {
        Op::MoveCopy
    }
}

fn random_file_size(rng: &mut impl Rng) -> usize {
    let lo: usize = 1024;
    let hi: usize = 102_400;
    rng.random_range(lo..=hi)
}

// ── Server helpers (mirrors rclone_e2e.rs) ───────────────────────────

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn spawn_server(port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_ferro-server"))
        .env("RUST_LOG", "warn")
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ferro-server (run `cargo build -p ferro-server` first)")
}

async fn wait_for_server(port: u16, max_wait: Duration) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let url = format!("http://127.0.0.1:{}/.well-known/ferro", port);
    let start = Instant::now();
    loop {
        if client
            .get(&url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            return;
        }
        if start.elapsed() > max_wait {
            panic!("Server did not start within {:?}", max_wait);
        }
        sleep(Duration::from_millis(100)).await;
    }
}

// ── /proc/PID/status memory reader ───────────────────────────────────

#[derive(Clone, Copy, Serialize)]
struct MemSnapshot {
    vm_rss_kb: u64,
    vm_size_kb: u64,
    elapsed_secs: f64,
}

fn read_memory_kb(pid: u32) -> Option<(u64, u64)> {
    let path = format!("/proc/{}/status", pid);
    let content = std::fs::read_to_string(&path).ok()?;
    let mut vm_rss: Option<u64> = None;
    let mut vm_size: Option<u64> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            vm_rss = rest.split_whitespace().next()?.parse().ok();
        } else if let Some(rest) = line.strip_prefix("VmSize:") {
            vm_size = rest.split_whitespace().next()?.parse().ok();
        }
    }
    Some((vm_rss?, vm_size?))
}

// ── Stats collection ─────────────────────────────────────────────────

#[derive(Default, Serialize)]
struct OpStats {
    total: u64,
    success: u64,
    failure: u64,
    latencies_ms: Vec<f64>,
}

#[derive(Serialize)]
struct SoakResults {
    duration_secs: u64,
    concurrent_users: usize,
    total_requests: u64,
    total_success: u64,
    total_failure: u64,
    requests_per_second: f64,
    overall_error_rate_pct: f64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    by_op: HashMap<String, OpStats>,
    memory_snapshots: Vec<MemSnapshot>,
    peak_rss_kb: u64,
    peak_vmsize_kb: u64,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn is_success(status: u16) -> bool {
    (200..300).contains(&status) || status == 207 || status == 304
}

// ── Worker ───────────────────────────────────────────────────────────

async fn worker(
    base_url: String,
    user_id: usize,
    stop: Arc<AtomicBool>,
    stats: Arc<std::sync::Mutex<HashMap<Op, OpStats>>>,
    total_ops: Arc<AtomicU64>,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let mut rng = StdRng::from_os_rng();
    let prefix = format!("/soak/u{}", user_id);
    let mut existing_files: Vec<String> = Vec::new();

    // Seed initial directory
    {
        let resp = client
            .request(
                Method::from_bytes(b"MKCOL").unwrap(),
                format!("{}{}", base_url, prefix),
            )
            .send()
            .await;
        let _ = resp;
    }

    while !stop.load(Ordering::Relaxed) {
        let op = pick_op(&mut rng);
        let start = Instant::now();
        let mut success = false;

        match op {
            Op::Put => {
                let size = random_file_size(&mut rng);
                let body: Vec<u8> = (0..size).map(|_| rng.random::<u8>()).collect();
                let fname = format!("file_{}", rng.random::<u64>());
                let path = format!("{}/{}", prefix, fname);
                if let Ok(resp) = client
                    .put(format!("{}{}", base_url, path))
                    .body(body)
                    .send()
                    .await
                {
                    success = is_success(resp.status().as_u16());
                    if success {
                        existing_files.push(path);
                        if existing_files.len() > 500 {
                            existing_files.remove(0);
                        }
                    }
                }
            }
            Op::Get => {
                let path = if !existing_files.is_empty() && rng.random_bool(0.9) {
                    let idx = rng.random_range(0..existing_files.len());
                    existing_files[idx].clone()
                } else {
                    format!("{}/nonexistent_{}", prefix, rng.random::<u64>())
                };
                if let Ok(resp) = client.get(format!("{}{}", base_url, path)).send().await {
                    success = is_success(resp.status().as_u16());
                }
            }
            Op::Propfind => {
                let path = if rng.random_bool(0.5) {
                    prefix.clone()
                } else if !existing_files.is_empty() {
                    let idx = rng.random_range(0..existing_files.len());
                    existing_files[idx].clone()
                } else {
                    prefix.clone()
                };
                if let Ok(resp) = client
                    .request(
                        Method::from_bytes(b"PROPFIND").unwrap(),
                        format!("{}{}", base_url, path),
                    )
                    .header("Depth", "1")
                    .send()
                    .await
                {
                    success = is_success(resp.status().as_u16());
                }
            }
            Op::Delete => {
                if !existing_files.is_empty() {
                    let idx = rng.random_range(0..existing_files.len());
                    let path = existing_files.remove(idx);
                    if let Ok(resp) = client.delete(format!("{}{}", base_url, path)).send().await {
                        success = is_success(resp.status().as_u16());
                    }
                } else {
                    success = true;
                }
            }
            Op::MoveCopy => {
                if existing_files.len() >= 2 {
                    let src_idx = rng.random_range(0..existing_files.len());
                    let src = existing_files[src_idx].clone();
                    let dest = format!("{}/mc_{}", prefix, rng.random::<u64>());
                    let method = if rng.random_bool(0.5) {
                        Method::from_bytes(b"MOVE").unwrap()
                    } else {
                        Method::from_bytes(b"COPY").unwrap()
                    };
                    let is_move = method == Method::from_bytes(b"MOVE").unwrap();
                    if let Ok(resp) = client
                        .request(method, format!("{}{}", base_url, src))
                        .header("Destination", format!("{}{}", base_url, dest))
                        .send()
                        .await
                    {
                        success = is_success(resp.status().as_u16());
                        if success {
                            existing_files.push(dest);
                            if is_move {
                                existing_files.remove(src_idx);
                            }
                        }
                    }
                } else {
                    success = true;
                }
            }
        }

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        total_ops.fetch_add(1, Ordering::Relaxed);

        {
            let mut map = stats.lock().unwrap();
            let entry = map.entry(op).or_default();
            entry.total += 1;
            if success {
                entry.success += 1;
            } else {
                entry.failure += 1;
            }
            entry.latencies_ms.push(latency_ms);
        }
    }
}

// ── Main test ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn soak_test() {
    let duration = soak_duration();
    let users = concurrent_users();
    let port = find_free_port();
    let mut server = spawn_server(port);
    let pid = server.id();

    eprintln!(
        "[soak] server PID={} port={} users={} duration={}s",
        pid,
        port,
        users,
        duration.as_secs()
    );

    wait_for_server(port, Duration::from_secs(15)).await;

    let base_url = format!("http://127.0.0.1:{}", port);
    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(std::sync::Mutex::new(HashMap::<Op, OpStats>::new()));
    let total_ops = Arc::new(AtomicU64::new(0));
    let mem_snapshots = Arc::new(std::sync::Mutex::new(Vec::<MemSnapshot>::new()));

    let start_time = Instant::now();

    // Memory sampler task
    let mem_stop = stop.clone();
    let mem_snaps = mem_snapshots.clone();
    let mem_handle = tokio::spawn(async move {
        loop {
            if mem_stop.load(Ordering::Relaxed) {
                break;
            }
            let elapsed = start_time.elapsed().as_secs_f64();
            if let Some((rss, vmsize)) = read_memory_kb(pid) {
                mem_snaps.lock().unwrap().push(MemSnapshot {
                    vm_rss_kb: rss,
                    vm_size_kb: vmsize,
                    elapsed_secs: elapsed,
                });
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    // Progress reporter
    let prog_stop = stop.clone();
    let prog_ops = total_ops.clone();
    let prog_start = start_time;
    let prog_handle = tokio::spawn(async move {
        loop {
            if prog_stop.load(Ordering::Relaxed) {
                break;
            }
            sleep(Duration::from_secs(30)).await;
            let elapsed = prog_start.elapsed().as_secs_f64();
            let ops = prog_ops.load(Ordering::Relaxed);
            let rps = ops as f64 / elapsed;
            eprintln!(
                "[soak] {:.0}s elapsed | {} ops | {:.1} req/s",
                elapsed, ops, rps
            );
        }
    });

    // Spawn workers
    let mut handles = Vec::with_capacity(users);
    for uid in 0..users {
        let h = tokio::spawn(worker(
            base_url.clone(),
            uid,
            stop.clone(),
            stats.clone(),
            total_ops.clone(),
        ));
        handles.push(h);
    }

    // Run for the configured duration
    sleep(duration).await;
    stop.store(true, Ordering::SeqCst);

    // Wait for workers
    for h in handles {
        let _ = h.await;
    }
    let _ = mem_handle.await;
    let _ = prog_handle.await;

    let wall_secs = start_time.elapsed().as_secs_f64();
    let total = total_ops.load(Ordering::Relaxed);

    // Collect per-op stats
    let stats_map = stats.lock().unwrap();
    let mut all_latencies: Vec<f64> = Vec::new();
    let mut total_success: u64 = 0;
    let mut total_failure: u64 = 0;
    let mut by_op_serializable: HashMap<String, OpStats> = HashMap::new();

    for (op, s) in stats_map.iter() {
        all_latencies.extend_from_slice(&s.latencies_ms);
        total_success += s.success;
        total_failure += s.failure;
        let mut op_s = OpStats {
            total: s.total,
            success: s.success,
            failure: s.failure,
            latencies_ms: Vec::new(),
        };
        let mut lats = s.latencies_ms.clone();
        lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
        op_s.latencies_ms = lats;
        by_op_serializable.insert(format!("{:?}", op), op_s);
    }

    all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    drop(stats_map);

    let rps = total as f64 / wall_secs;
    let error_rate = if total > 0 {
        total_failure as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let p50 = percentile(&all_latencies, 50.0);
    let p95 = percentile(&all_latencies, 95.0);
    let p99 = percentile(&all_latencies, 99.0);

    // Memory peaks
    let mem = mem_snapshots.lock().unwrap();
    let peak_rss = mem.iter().map(|s| s.vm_rss_kb).max().unwrap_or(0);
    let peak_vmsize = mem.iter().map(|s| s.vm_size_kb).max().unwrap_or(0);
    let mem_for_results = mem.clone();
    drop(mem);

    let results = SoakResults {
        duration_secs: duration.as_secs(),
        concurrent_users: users,
        total_requests: total,
        total_success,
        total_failure,
        requests_per_second: rps,
        overall_error_rate_pct: error_rate,
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
        by_op: by_op_serializable,
        memory_snapshots: mem_for_results,
        peak_rss_kb: peak_rss,
        peak_vmsize_kb: peak_vmsize,
    };

    // Write JSON results
    let output_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target");
    let _ = std::fs::create_dir_all(&output_dir);
    let output_path = output_dir.join("soak-results.json");
    let json = serde_json::to_string_pretty(&results).unwrap();
    std::fs::write(&output_path, &json).expect("failed to write soak-results.json");

    // Print summary
    eprintln!();
    eprintln!("══════════════════════════════════════════════════════");
    eprintln!("  SOAK TEST RESULTS");
    eprintln!("══════════════════════════════════════════════════════");
    eprintln!("  Duration:       {:.1}s", wall_secs);
    eprintln!("  Concurrent:     {} users", users);
    eprintln!("  Total requests: {}", total);
    eprintln!("  Success:        {}", total_success);
    eprintln!("  Failure:        {}", total_failure);
    eprintln!("  Req/sec:        {:.1}", rps);
    eprintln!("  Error rate:     {:.2}%", error_rate);
    eprintln!("  Latency P50:    {:.2}ms", p50);
    eprintln!("  Latency P95:    {:.2}ms", p95);
    eprintln!("  Latency P99:    {:.2}ms", p99);
    eprintln!("  Peak RSS:       {} KB", peak_rss);
    eprintln!("  Peak VMSize:    {} KB", peak_vmsize);
    eprintln!("  Results:        {}", output_path.display());
    eprintln!("══════════════════════════════════════════════════════");

    // Per-op breakdown
    eprintln!();
    eprintln!(
        "  {:<12} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10}",
        "OP", "TOTAL", "OK", "FAIL", "ERR%", "P50", "P95", "P99"
    );
    for (op, s) in &results.by_op {
        let err_pct = if s.total > 0 {
            s.failure as f64 / s.total as f64 * 100.0
        } else {
            0.0
        };
        let op_p50 = percentile(&s.latencies_ms, 50.0);
        let op_p95 = percentile(&s.latencies_ms, 95.0);
        let op_p99 = percentile(&s.latencies_ms, 99.0);
        eprintln!(
            "  {:<12} {:>8} {:>8} {:>8} {:>7.2}% {:>9.2}ms {:>9.2}ms {:>9.2}ms",
            op, s.total, s.success, s.failure, err_pct, op_p50, op_p95, op_p99
        );
    }

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    // Failure thresholds
    let mut failed = false;

    if error_rate > 1.0 {
        eprintln!(
            "\n[FAIL] Error rate {:.2}% exceeds 1% threshold",
            error_rate
        );
        failed = true;
    }

    // P99 for small file ops (PUT, GET) must be < 100ms
    for op_name in &["Put", "Get"] {
        if let Some(s) = results.by_op.get(*op_name) {
            let op_p99 = percentile(&s.latencies_ms, 99.0);
            if op_p99 > 100.0 {
                eprintln!(
                    "\n[FAIL] {} P99 {:.2}ms exceeds 100ms threshold",
                    op_name, op_p99
                );
                failed = true;
            }
        }
    }

    assert!(!failed, "Soak test failed — see above for details");
}
