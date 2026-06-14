# PerfMax

Real-time AI system performance optimizer. Monitors CPU/GPU metrics, captures active workload context, and uses a local LLM (MiMo-7B-RL) to generate and apply tuning commands continuously.

## Stack

- **Rust** — egui (wgpu) native GUI + system tray (ksni/KDE SNI)
- **AI** — local LLM via OpenAI-compat endpoint (MiMo-7B-RL on llama.cpp, port 8081)
- **Metrics** — sysinfo (CPU/processes) + NVML (GPU)
- **Optimizer** — ryzenadj, nvidia-smi, cpupower, sysctl, taskset

## Install

```bash
./install.sh
```

Builds release binary, installs to `~/.local/bin`, registers systemd user service, and writes sudoers rules for the tuning commands.

## AI model setup

```bash
# Download MiMo-7B-RL GGUF
huggingface-cli download jedisct1/MiMo-7B-RL-GGUF \
  --include "*.Q8_0.gguf" --local-dir ~/models/mimo-7b

# Serve on port 8081
llama-server --model ~/models/mimo-7b/MiMo-7B-RL-Q8_0.gguf \
  --port 8081 --n-gpu-layers 999 --ctx-size 8192
```

Or point Settings → Endpoint at your existing vLLM instance (e.g. `http://127.0.0.1:8080/v1`, model `aeon`).

## Usage

```bash
perfmax
# or with debug logging
RUST_LOG=perfmax=debug perfmax
```

**Enable dry-run mode first** (Settings tab) to review what commands the AI generates before letting it execute.

Tray menu: Show/Hide · Optimize Now · Quit

## What it tunes

| Category | Commands |
|----------|---------|
| CPU | ryzenadj (TDP/PBO limits), cpupower governor, CCD online/offline |
| GPU | nvidia-smi power limit, clock lock |
| Scheduler | sysctl sched_*, taskset affinity, renice |
| Memory | vm.swappiness, hugepages, THP |
| I/O | block scheduler per device |
