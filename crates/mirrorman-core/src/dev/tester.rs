use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::models::DevMirror;

const USER_AGENT: &str = "mirrorman/0.5.3";

/// Tests a single endpoint with a GET request, returning response latency in milliseconds.
pub fn test_single_endpoint(client: &reqwest::blocking::Client, ping_url: &str) -> Option<f64> {
    let start = Instant::now();
    match client.get(ping_url).send() {
        Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            Some(elapsed_ms)
        }
        _ => None,
    }
}

/// Concurrently tests candidate mirrors using a worker thread pool.
pub fn test_dev_mirrors(mirrors: &mut [DevMirror], max_workers: usize) {
    if mirrors.is_empty() {
        return;
    }

    let num_workers = max_workers.max(1).min(mirrors.len());
    let queue: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(
        mirrors
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.ping_url.is_empty())
            .map(|(idx, m)| (idx, m.ping_url.clone()))
            .collect(),
    ));

    let results: Arc<Mutex<Vec<(usize, Option<f64>)>>> = Arc::new(Mutex::new(Vec::new()));

    let client = match reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => Arc::new(c),
        Err(_) => return,
    };

    let mut handles = Vec::with_capacity(num_workers);
    for _ in 0..num_workers {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        let client = Arc::clone(&client);

        let handle = std::thread::spawn(move || loop {
            let item = {
                let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
                q.pop()
            };
            let (idx, url) = match item {
                Some(val) => val,
                None => break,
            };

            let speed = test_single_endpoint(&client, &url);
            results
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((idx, speed));
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.join();
    }

    let final_results = results.lock().unwrap_or_else(|e| e.into_inner());
    for &(idx, speed) in final_results.iter() {
        if idx < mirrors.len() {
            mirrors[idx].speed = speed;
        }
    }

    // Sort by latency: fastest (lowest ms) first, failed/unreachable at the end
    mirrors.sort_by(|a, b| match (a.speed, b.speed) {
        (Some(sa), Some(sb)) => sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}
