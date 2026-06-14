use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    pub usage_per_core: Vec<f32>,
    pub total_usage: f32,
    pub frequency_mhz: Vec<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub utilization_pct: u32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub power_draw_w: f32,
    pub power_limit_w: f32,
    pub temperature: u32,
    pub clock_sm_mhz: u32,
    pub clock_mem_mhz: u32,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_mb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct AiState {
    pub last_recommendation: String,
    pub last_commands: Vec<String>,
    pub thinking: bool,
    pub last_updated: Option<Instant>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub cpu: Option<CpuMetrics>,
    pub gpu: Option<GpuMetrics>,
    pub top_processes: Vec<ProcessInfo>,
    pub active_window: String,
    pub ai: AiState,
    pub pending_commands: Vec<String>,
    pub command_history: VecDeque<String>,
    pub optimize_trigger: bool,
    /// provider name → (online, [model_ids])
    pub provider_status: HashMap<String, (bool, Vec<String>)>,
    pub model_download: Option<DownloadState>,
    pub local_server_online: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadState {
    pub file: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub done: bool,
    pub error: Option<String>,
}

impl AppState {
    pub fn push_history(&mut self, entry: String) {
        self.command_history.push_front(entry);
        if self.command_history.len() > 200 {
            self.command_history.pop_back();
        }
    }
}
