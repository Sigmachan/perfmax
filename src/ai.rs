use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::{Config, Provider};
use crate::state::AppState;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

const SYSTEM_PROMPT: &str = "\
You are a Linux system performance optimizer for a high-end workstation.\
Output ONLY shell commands, one per line. No markdown, no explanation, no backticks.\
Only use the allowed commands listed in the user message.";

pub async fn run(state: Arc<RwLock<AppState>>, config: Config) {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("reqwest client");

    let interval = Duration::from_secs(config.ai.interval_secs);

    loop {
        // Wait for interval or manual trigger
        let mut waited = Duration::ZERO;
        let tick = Duration::from_millis(500);
        loop {
            tokio::time::sleep(tick).await;
            waited += tick;
            let triggered = {
                let mut s = state.write().unwrap();
                if s.optimize_trigger {
                    s.optimize_trigger = false;
                    true
                } else {
                    false
                }
            };
            if triggered || waited >= interval {
                break;
            }
        }

        let snapshot = {
            let s = state.read().unwrap();
            build_snapshot(&s)
        };

        if let Ok(mut s) = state.write() {
            s.ai.thinking = true;
            s.ai.error = None;
        }

        let provider = match config.ai.active_provider() {
            Some(p) => p.clone(),
            None => {
                tracing::warn!("No active provider configured");
                continue;
            }
        };

        info!("Querying AI via '{}' ({})...", provider.name, provider.endpoint);

        match query_ai(&client, &provider, config.ai.max_tokens, snapshot).await {
            Ok(response) => {
                let commands = parse_commands(&response);
                info!("AI returned {} commands", commands.len());
                if let Ok(mut s) = state.write() {
                    s.ai.thinking = false;
                    s.ai.last_recommendation = response;
                    s.ai.last_commands = commands.clone();
                    s.ai.last_updated = Some(Instant::now());
                    if config.optimizer.enabled {
                        s.pending_commands.extend(commands);
                    }
                }
            }
            Err(e) => {
                warn!("AI error: {e}");
                if let Ok(mut s) = state.write() {
                    s.ai.thinking = false;
                    s.ai.error = Some(e.to_string());
                }
            }
        }
    }
}

fn build_snapshot(state: &AppState) -> String {
    let cpu_str = state.cpu.as_ref().map(|c| {
        let avg_freq = if c.frequency_mhz.is_empty() {
            0
        } else {
            c.frequency_mhz.iter().sum::<u64>() / c.frequency_mhz.len() as u64
        };
        format!(
            "CPU total={:.1}% avg_freq={}MHz cores=[{}]",
            c.total_usage,
            avg_freq,
            c.usage_per_core
                .iter()
                .map(|x| format!("{:.0}", x))
                .collect::<Vec<_>>()
                .join(",")
        )
    }).unwrap_or_else(|| "CPU: no data".into());

    let gpu_str = state.gpu.as_ref().map(|g| {
        format!(
            "GPU util={}% VRAM={}/{}MB power={:.0}/{:.0}W temp={}C sm={}MHz mem={}MHz",
            g.utilization_pct,
            g.memory_used_mb,
            g.memory_total_mb,
            g.power_draw_w,
            g.power_limit_w,
            g.temperature,
            g.clock_sm_mhz,
            g.clock_mem_mhz,
        )
    }).unwrap_or_else(|| "GPU: no data".into());

    let procs_str = state
        .top_processes
        .iter()
        .take(10)
        .map(|p| format!("  {}({}): {:.1}%cpu", p.name, p.pid, p.cpu_pct))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"Hardware: Ryzen 9 9950X3D (16c/32t, CCD0=8c X3D-VCache gaming CCD, CCD1=8c standard), RTX 5090 32GB Blackwell, 64GB DDR5, CachyOS linux-tkg BORE/EEVDF 1000Hz.
{cpu_str}
{gpu_str}
Active window: {win}
Top processes by CPU:
{procs_str}

Allowed commands ONLY:
  ryzenadj --stapm-limit=<mW> --fast-limit=<mW> --slow-limit=<mW>
  nvidia-smi -pl <watts>
  nvidia-smi -lgc <min_mhz>,<max_mhz>
  nvidia-smi -pm 0|1
  cpupower frequency-set -g performance|schedutil|powersave
  echo 0|1 > /sys/devices/system/cpu/cpu<N>/online
  sysctl -w vm.swappiness=<N>
  sysctl -w vm.nr_hugepages=<N>
  sysctl -w kernel.sched_latency_ns=<N>
  taskset -cp <cpu_list> <pid>
  renice -n <-20..19> -p <pid>
  ionice -c 1 -p <pid>
  echo mq-deadline|none|kyber > /sys/block/<dev>/queue/scheduler

Output ONLY the commands. One per line. No explanation."#,
        win = state.active_window,
    )
}

async fn query_ai(
    client: &Client,
    provider: &crate::config::Provider,
    max_tokens: u32,
    prompt: String,
) -> anyhow::Result<String> {
    let body = ChatRequest {
        model: provider.model.clone(),
        messages: vec![
            Message { role: "system".into(), content: SYSTEM_PROMPT.into() },
            Message { role: "user".into(), content: prompt },
        ],
        max_tokens,
        temperature: 0.1,
        stream: false,
    };

    let mut req = client
        .post(format!("{}/chat/completions", provider.endpoint))
        .json(&body);

    if let Some(key) = &provider.api_key {
        req = req.bearer_auth(key);
    }

    let resp = req
        .send()
        .await?
        .error_for_status()?
        .json::<ChatResponse>()
        .await?;

    Ok(resp
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default())
}

fn parse_commands(text: &str) -> Vec<String> {
    let allowed_prefixes = [
        "ryzenadj",
        "nvidia-smi",
        "cpupower",
        "echo",
        "sysctl",
        "taskset",
        "renice",
        "ionice",
    ];

    text.lines()
        .map(|l| l.trim().trim_start_matches('$').trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("```"))
        .filter(|l| allowed_prefixes.iter().any(|p| l.starts_with(p)))
        .collect()
}
