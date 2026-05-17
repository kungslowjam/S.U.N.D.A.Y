//! Cold-start orchestrator — boots AI engine, backend, and frontend for E2E testing.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Manages the lifecycle of SUNDAY services for harness testing.
pub struct BootOrchestrator {
    managed_processes: Vec<(String, Child)>,
    is_windows: bool,
}

impl BootOrchestrator {
    pub fn new() -> Self {
        Self {
            managed_processes: Vec::new(),
            is_windows: cfg!(target_os = "windows"),
        }
    }

    /// Full cold-start: AI engine → backend → frontend.
    pub async fn cold_start(
        &mut self,
        llama_port: u16,
        backend_port: u16,
        frontend_port: u16,
        model_path: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("🚀 Starting SUNDAY harness cold-start...");

        // Phase 0: Pre-clean
        self.pre_clean();

        // Phase 1: Start AI Engine (llama-server)
        if !Self::is_port_in_use(llama_port) {
            tracing::info!("🧠 Starting AI engine on port {}...", llama_port);
            self.start_llama_server(llama_port, model_path, None)?;
            if !Self::wait_for_port(llama_port, 60).await {
                return Err("AI engine failed to start".into());
            }
        } else {
            tracing::info!("✅ AI engine already running on port {}", llama_port);
        }

        // Phase 2: Start Backend
        if !Self::is_port_in_use(backend_port) {
            tracing::info!("🖥️  Starting backend on port {}...", backend_port);
            self.start_backend(backend_port)?;
            if !Self::wait_for_port(backend_port, 30).await {
                return Err("Backend failed to start".into());
            }
            // Wait for /health
            if !Self::wait_for_health(backend_port, 30).await {
                return Err("Backend health check failed".into());
            }
        } else {
            tracing::info!("✅ Backend already running on port {}", backend_port);
        }

        // Phase 3: Start Frontend
        if !Self::is_port_in_use(frontend_port) {
            tracing::info!("🌐 Starting frontend on port {}...", frontend_port);
            self.start_frontend(frontend_port)?;
            if !Self::wait_for_frontend(frontend_port, 30).await {
                return Err("Frontend failed to start".into());
            }
        } else {
            tracing::info!("✅ Frontend already running on port {}", frontend_port);
        }

        tracing::info!("🎉 All services ready!");
        Ok(())
    }

    /// Kill lingering processes before starting.
    fn pre_clean(&self) {
        tracing::info!("🧹 Pre-cleaning lingering processes...");
        let targets = if self.is_windows {
            vec!["llama-server.exe", "node.exe"]
        } else {
            vec!["llama-server", "node"]
        };

        for target in targets {
            self.kill_process_by_name(target);
        }
    }

    fn start_llama_server(
        &mut self,
        port: u16,
        model_path: Option<PathBuf>,
        gpu_layers: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_root = PathBuf::from(".");
        let llama_exe = if self.is_windows {
            project_root.join("llama-cpp/llama-server.exe")
        } else {
            project_root.join("llama-cpp/llama-server")
        };

        let model = model_path.unwrap_or_else(|| {
            project_root.join("llama-cpp/models/Qwen3.5-9B-DeepSeek-V4-Flash-Q4_K_S.gguf")
        });

        let ngl = gpu_layers.unwrap_or_else(|| {
            std::env::var("SUNDAY_GPU_LAYERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(35)
        });
        let parallel = std::env::var("SUNDAY_HARNESS_PARALLEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let mut cmd = Command::new(&llama_exe);
        cmd.arg("-m").arg(&model)
            .arg("--port").arg(port.to_string())
            .arg("-ngl").arg(ngl.to_string())
            .arg("-c").arg("32768")
            .arg("-np").arg(parallel.to_string())
            .arg("--host").arg("127.0.0.1")
            .env("SUNDAY_HARNESS_MODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.managed_processes.push(("AI-ENGINE".to_string(), child));
        Ok(())
    }

    fn start_backend(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = if self.is_windows {
            let mut c = Command::new("cmd");
            c.arg("/C")
                .arg(".venv/Scripts/python.exe")
                .arg("-m")
                .arg("sunday.cli")
                .arg("serve")
                .arg("--port")
                .arg(port.to_string());
            c
        } else {
            let mut c = Command::new(".venv/bin/python");
            c.arg("-m")
                .arg("sunday.cli")
                .arg("serve")
                .arg("--port")
                .arg(port.to_string());
            c
        };

        cmd.env("SUNDAY_HARNESS_MODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.managed_processes.push(("BACKEND".to_string(), child));
        Ok(())
    }

    fn start_frontend(&mut self, _port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("npm");
        cmd.arg("run")
            .arg("dev")
            .current_dir("frontend")
            .env("SUNDAY_HARNESS_MODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.managed_processes.push(("FRONTEND".to_string(), child));
        Ok(())
    }

    async fn wait_for_port(port: u16, timeout_sec: u64) -> bool {
        let start = Instant::now();
        while start.elapsed().as_secs() < timeout_sec {
            if Self::is_port_in_use(port) {
                return true;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        false
    }

    async fn wait_for_health(port: u16, timeout_sec: u64) -> bool {
        let url = format!("http://127.0.0.1:{}/health", port);
        let start = Instant::now();
        while start.elapsed().as_secs() < timeout_sec {
            match reqwest::get(&url).await {
                Ok(res) if res.status().is_success() => return true,
                _ => tokio::time::sleep(Duration::from_secs(1)).await,
            }
        }
        false
    }

    async fn wait_for_frontend(port: u16, timeout_sec: u64) -> bool {
        let hosts = ["localhost", "127.0.0.1"];
        let start = Instant::now();
        while start.elapsed().as_secs() < timeout_sec {
            for host in &hosts {
                let url = format!("http://{}:{}", host, port);
                match reqwest::get(&url).await {
                    Ok(res) => {
                        if let Ok(body) = res.text().await {
                            let lower = body.to_lowercase();
                            if lower.contains("vite") || lower.contains("<div id=\"root\"") {
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        false
    }

    fn is_port_in_use(port: u16) -> bool {
        TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", port).parse().unwrap(),
            Duration::from_millis(500),
        )
        .is_ok()
    }

    fn kill_process_by_name(&self, name: &str) {
        if self.is_windows {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", name])
                .output();
        } else {
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(name)
                .output();
        }
    }
}

impl Drop for BootOrchestrator {
    fn drop(&mut self) {
        tracing::info!("🛑 Shutting down managed processes...");
        for (name, mut child) in self.managed_processes.drain(..) {
            tracing::info!("  Stopping {} (PID: {:?})", name, child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
