use std::sync::{Arc, RwLock};
use std::time::Duration;
use sysinfo::System;
use tracing::debug;

use crate::state::{AppState, CpuMetrics, GpuMetrics, ProcessInfo};

pub async fn run(state: Arc<RwLock<AppState>>, interval_ms: u64) {
    let mut sys = System::new_all();

    let nvml = nvml_wrapper::Nvml::init()
        .map_err(|e| tracing::warn!("NVML unavailable: {e}"))
        .ok();

    loop {
        sys.refresh_all();

        let cpus = sys.cpus();
        let cpu = CpuMetrics {
            usage_per_core: cpus.iter().map(|c| c.cpu_usage()).collect(),
            total_usage: if cpus.is_empty() {
                0.0
            } else {
                cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
            },
            frequency_mhz: cpus.iter().map(|c| c.frequency()).collect(),
        };

        let gpu = nvml.as_ref().and_then(|n| {
            let dev = n.device_by_index(0).ok()?;
            let util = dev.utilization_rates().ok()?;
            let mem = dev.memory_info().ok()?;
            let power = dev.power_usage().ok()?;
            let power_limit = dev.enforced_power_limit().ok()?;
            let temp = dev
                .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .unwrap_or(0);
            let clock_sm = dev
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::SM)
                .unwrap_or(0);
            let clock_mem = dev
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
                .unwrap_or(0);

            Some(GpuMetrics {
                utilization_pct: util.gpu,
                memory_used_mb: mem.used / 1024 / 1024,
                memory_total_mb: mem.total / 1024 / 1024,
                power_draw_w: power as f32 / 1000.0,
                power_limit_w: power_limit as f32 / 1000.0,
                temperature: temp,
                clock_sm_mhz: clock_sm,
                clock_mem_mhz: clock_mem,
            })
        });

        let mut procs: Vec<ProcessInfo> = sys
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu_pct: p.cpu_usage(),
                mem_mb: p.memory() / 1024 / 1024,
            })
            .collect();
        procs.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs.truncate(20);

        let active_window = get_active_window();

        if let Ok(mut s) = state.write() {
            s.cpu = Some(cpu);
            s.gpu = gpu;
            s.top_processes = procs;
            s.active_window = active_window;
        }

        debug!("Metrics refreshed");
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
}

fn get_active_window() -> String {
    std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}
