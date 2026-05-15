use clap::{Parser, Subcommand};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use sysinfo::System;

mod tui;

#[derive(Parser)]
#[command(name = "sunday")]
#[command(about = "SUNDAY - Autonomous Agent CLI & Process Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the status of all SUNDAY services
    Status,
    /// Start SUNDAY services (Llama-Server + Backend + Frontend + Sidecars)
    Start {
        #[arg(long, default_value = "8081")]
        llama_port: u16,
        #[arg(long, default_value = "8000")]
        backend_port: u16,
        #[arg(long, default_value = "5173")]
        frontend_port: u16,
        #[arg(long)]
        model_path: Option<String>,
        /// Force restart services if they are already running
        #[arg(short, long)]
        force: bool,
        /// Start Discord Daemon
        #[arg(long)]
        discord: bool,
        /// Start Voice Live Overlay
        #[arg(long)]
        voice: bool,
        /// Start everything (Core + Discord + Voice)
        #[arg(short, long)]
        all: bool,
    },
    /// Stop all SUNDAY services by clearing ports
    Stop,
    /// Run system diagnostics to check health and environment
    Doctor,
    /// Build Rust bridge components using maturin
    Build,
    /// Open the TUI Dashboard
    Dashboard,
    /// Manage AI models (list, download, remove)
    Models {
        #[command(subcommand)]
        action: ModelAction,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available and downloaded models
    List,
    /// Download a recommended model
    Download {
        /// Name of the model to download (e.g., 'qwen-9b')
        name: Option<String>,
        /// Custom URL to download from
        #[arg(long)]
        url: Option<String>,
    },
    /// Remove a model from local storage
    Remove {
        name: String,
    },
}

pub fn check_port(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    if let Ok(socket_addr) = addr.parse() {
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)).is_ok()
    } else {
        false
    }
}

fn wait_for_port(port: u16, timeout_sec: u64) -> bool {
    let start = Instant::now();
    print!("Waiting for port {} ", port);
    while start.elapsed().as_secs() < timeout_sec {
        if check_port(port) {
            println!(" [OK]");
            return true;
        }
        print!(".");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
    }
    println!(" [TIMEOUT]");
    false
}

fn stop_port(port: u16) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let mut found = false;
    // On Windows, finding which process owns a port natively is easier with external tools
    let output = Command::new("cmd")
        .args(&["/C", &format!("for /f \"tokens=5\" %a in ('netstat -aon ^| findstr :{} ^| findstr LISTENING') do taskkill /F /PID %a /T", port)])
        .output();

    if let Ok(out) = output {
        if !out.stdout.is_empty() {
            println!("  Port {} cleared via taskkill.", port);
            found = true;
        }
    }

    // Second, safety check: kill any remaining "zombie" processes by name if they relate to SUNDAY
    let targets = ["llama-server.exe", "uvicorn.exe", "node.exe"];
    for target in targets {
        for process in sys.processes_by_exact_name(std::ffi::OsStr::new(target)) {
            println!("  Cleaning up rogue {} (PID: {})", target, process.pid());
            let _ = process.kill();
            found = true;
        }
    }

    found
}

async fn download_model(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use indicatif::{ProgressBar, ProgressStyle};
    use futures_util::StreamExt;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600)) // 1 hour timeout for large models
        .build()?;
        
    let res = client.get(url).send().await?;
    let total_size = res.content_length().ok_or("Failed to get content length from server. The URL might be invalid or the server doesn't support content-length.")?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
    
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    pb.set_message(format!("Downloading {}", file_name));

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::File::create(path)?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        std::io::copy(&mut chunk.as_ref(), &mut file)?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Download complete! Model saved to {}", path.display()));
    Ok(())
}

fn get_project_root() -> PathBuf {
    let current = std::env::current_dir().unwrap();
    if current.join("src").exists() && current.join("rust").exists() {
        current
    } else if let Some(parent) = current.parent() {
        parent.to_path_buf()
    } else {
        current
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let root = get_project_root();

    match &cli.command {
        Commands::Status => {
            println!("SUNDAY System Status:");
            println!("---------------------");

            let ports = [
                ("Llama-Server", 8081),
                ("Backend API", 8000),
                ("Frontend", 5173),
                ("Voice-Live", 8098),
            ];

            for (name, port) in ports {
                let status = if check_port(port) {
                    "\x1b[32mRUNNING\x1b[0m"
                } else {
                    "\x1b[31mSTOPPED\x1b[0m"
                };
                println!("{:<15} : {}", name, status);
            }

            println!("\nActive Processes:");
            let sys = System::new_all();

            let target_names = ["llama-server", "python", "node"];
            for (pid, process) in sys.processes() {
                let name = process.name().to_string_lossy().to_lowercase();
                let cmd_parts: Vec<String> = process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect();
                let cmd = cmd_parts.join(" ");
                
                if target_names.iter().any(|&t| name.contains(t)) {
                    if cmd.contains("sunday") || cmd.contains("llama") || cmd.contains("vite") {
                        println!("  [{}] (PID: {})", name, pid);
                        if !cmd.is_empty() {
                            println!("       Cmd: {}", cmd);
                        }
                    }
                }
            }
        }

        Commands::Stop => {
            println!("Stopping all SUNDAY services...");
            let ports = [8081, 8000, 5173, 8082, 8098];
            for port in ports {
                stop_port(port);
            }
            println!("All cleared. ✨");
        }

        Commands::Start { llama_port, backend_port, frontend_port, model_path, force, discord, voice, all } => {
            let start_discord = *discord || *all;
            let start_voice = *voice || *all;
            
            // Load config to check default engine
            let config_path = root.join("configs").join("sunday").join("config.toml");
            let config = sunday_core::config::load_config(Some(&config_path)).unwrap_or_default();
            let engine_type = config.engine.default.clone();
            let is_native = engine_type == "native";

            if *force {
                println!("Force restart enabled. Clearing ports...");
                if !is_native {
                    stop_port(*llama_port);
                }
                stop_port(*backend_port);
                stop_port(*frontend_port);
            }

            println!("Starting SUNDAY Services (Engine: {})...", engine_type);
            
            // 1. Start Llama-Server (Only if NOT native)
            if is_native {
                println!("[1/5] Native Engine detected. Skipping llama-server startup.");
            } else if check_port(*llama_port) {
                println!("[1/5] [SKIP] AI Engine is already running on port {}", llama_port);
            } else {
                println!("[1/5] Launching AI Engine (Legacy HTTP)...");
                let llama_exe = root.join("llama-cpp").join("llama-server.exe");
                let model = model_path.clone().unwrap_or_else(|| {
                    if !config.intelligence.model_path.is_empty() {
                        config.intelligence.model_path.clone()
                    } else {
                        "llama-cpp/models/Qwen3.5-9B-DeepSeek-V4-Flash-Q4_K_S.gguf".to_string()
                    }
                });
                
                let model_full_path = if Path::new(&model).is_absolute() {
                    PathBuf::from(&model)
                } else {
                    root.join(&model)
                };

                if !model_full_path.exists() {
                    println!("  \x1b[31m[ERROR]\x1b[0m Model file not found at {:?}", model_full_path);
                    return;
                }

                let child = Command::new(&llama_exe)
                    .args(&[
                        "-m", model_full_path.to_str().unwrap(),
                        "--port", &llama_port.to_string(),
                        "-ngl", "35",
                        "-c", "16384",
                    ])
                    .current_dir(root.join("llama-cpp"))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to start llama-server");
                
                println!("      AI Engine started (PID: {})", child.id());
                wait_for_port(*llama_port, 120);
            }

            // 2. Start Backend
            if check_port(*backend_port) {
                println!("[2/5] [SKIP] SUNDAY Backend is already running on port {}", backend_port);
            } else {
                println!("[2/5] Launching SUNDAY Backend...");
                let python_exe = root.join(".venv").join("Scripts").join("python.exe");
                
                let child = Command::new(&python_exe)
                    .args(&[
                        "-m", "sunday", "serve",
                        "--engine", &engine_type,
                        "--agent", "orchestrator",
                        "--port", &backend_port.to_string(),
                    ])
                    .env("PYTHONPATH", root.join("src").to_str().unwrap())
                    .env("OPENSUNDAY_CONFIG", config_path.to_str().unwrap())
                    .current_dir(&root)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to start SUNDAY backend");

                println!("      Backend started (PID: {})", child.id());
                wait_for_port(*backend_port, 120);
            }

            // 3. Start Frontend
            if check_port(*frontend_port) {
                println!("[SKIP] Frontend is already running on port {}", frontend_port);
            } else {
                println!("[3/5] Launching Frontend Dashboard...");
                let child = Command::new("cmd")
                    .args(&["/C", "npm run dev"])
                    .current_dir(root.join("frontend"))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to start frontend");

                println!("      Frontend started (PID: {})", child.id());
                if !wait_for_port(*frontend_port, 120) {
                    eprintln!("      [WARN] Frontend port {} not ready after 60s, trying alternate port check...", frontend_port);
                    // Vite may use a different port if 5173 is taken
                    std::thread::sleep(Duration::from_secs(2));
                }
            }

            // 4. Start Voice Live (Optional)
            if start_voice {
                println!("[4/5] Launching Voice Live Overlay...");
                let voice_ps = root.join("voice-live").join("start_voice_live.ps1");
                let _ = Command::new("powershell")
                    .args(&["-File", voice_ps.to_str().unwrap()])
                    .current_dir(root.join("voice-live"))
                    .spawn();
                println!("      Voice Live started.");
            }

            // 5. Start Discord (Optional)
            if start_discord {
                println!("[5/5] Launching Discord Daemon (Rust)...");
                let discord_exe = root.join("rust").join("target").join("release").join("sunday-discord.exe");
                
                let _ = if discord_exe.exists() {
                    Command::new(&discord_exe).spawn()
                } else {
                    // Fallback to cargo run for development
                    Command::new("cargo")
                        .args(&["run", "--release", "-p", "sunday-discord"])
                        .current_dir(root.join("rust"))
                        .spawn()
                };
                println!("      Discord Daemon started.");
            }

            println!("\nAll systems are go! 🚀");
            println!("Dashboard: http://localhost:5173");
            // Automatically open browser on Windows
            let _ = Command::new("cmd")
                .args(&["/C", "start", "http://localhost:5173"])
                .spawn();
        }
        Commands::Build => {
            println!("Building SUNDAY Rust Bridge...");
            let maturin_exe = root.join(".venv").join("Scripts").join("maturin.exe");
            if !maturin_exe.exists() {
                println!("  \x1b[31m[ERROR]\x1b[0m maturin not found in .venv. Run 'uv sync --extra rust'");
                return;
            }

            let status = Command::new(&maturin_exe)
                .args(&["develop", "--release", "-m", "crates/sunday-python/Cargo.toml"])
                .current_dir(root.join("rust"))
                .status()
                .expect("Failed to run maturin");

            if status.success() {
                println!("  \x1b[32m[OK]\x1b[0m Rust bridge built and installed.");
            } else {
                println!("  \x1b[31m[FAIL]\x1b[0m maturin build failed.");
            }
        }
        Commands::Dashboard => {
            if let Err(e) = tui::run_dashboard() {
                eprintln!("Dashboard error: {}", e);
            }
        }
        Commands::Doctor => {
            println!("SUNDAY System Doctor 🩺");
            println!("======================");
            
            // 1. Check Directories
            let root = get_project_root();
            println!("[ ] Checking project root: {}", root.display());
            
            let required_dirs = ["src", "rust", "llama-cpp", "frontend", "configs"];
            for dir in required_dirs {
                let p = root.join(dir);
                if p.exists() {
                    println!("  \x1b[32m[OK]\x1b[0m Directory '{}' found", dir);
                } else {
                    println!("  \x1b[31m[FAIL]\x1b[0m Directory '{}' missing!", dir);
                }
            }

            // 2. Check AI Engine
            let llama_exe = root.join("llama-cpp").join("llama-server.exe");
            if llama_exe.exists() {
                println!("  \x1b[32m[OK]\x1b[0m AI Engine (llama-server.exe) found");
            } else {
                println!("  \x1b[31m[FAIL]\x1b[0m AI Engine missing at {:?}", llama_exe);
            }

            // 3. Check Python Environment
            let python_exe = root.join(".venv").join("Scripts").join("python.exe");
            if python_exe.exists() {
                println!("  \x1b[32m[OK]\x1b[0m Python virtual environment found");
                
                // Check if sunday_rust is importable
                let output = Command::new(&python_exe)
                    .args(&["-c", "import sunday_rust; print('ok')"])
                    .output();
                
                if let Ok(out) = output {
                    if String::from_utf8_lossy(&out.stdout).trim() == "ok" {
                        println!("  \x1b[32m[OK]\x1b[0m Rust bridge (sunday_rust) is functional");
                    } else {
                        println!("  \x1b[31m[FAIL]\x1b[0m Rust bridge exists but failed to import");
                    }
                }
            } else {
                println!("  \x1b[31m[FAIL]\x1b[0m .venv missing! Run 'uv sync' or 'pip install -e .'");
            }

            // 4. Port Check
            println!("\n[ ] Port Status:");
            let ports = [
                ("Llama (8081)", 8081),
                ("Backend (8000)", 8000),
                ("Vite (5173)", 5173)
            ];
            for (label, port) in ports {
                if check_port(port) {
                    println!("  - {:<15} : \x1b[33mBUSY\x1b[0m (Service may be running)", label);
                } else {
                    println!("  - {:<15} : \x1b[32mFREE\x1b[0m", label);
                }
            }

            println!("\nDiagnosis complete. ✨");
        }
        Commands::Models { action } => {
            match action {
                ModelAction::List => {
                    println!("SUNDAY Model Inventory:");
                    println!("=======================");
                    let models_dir = root.join("llama-cpp").join("models");
                    if !models_dir.exists() {
                        println!("Models directory missing. Use 'sunday models download' to create it.");
                        return;
                    }

                    let entries = std::fs::read_dir(&models_dir).unwrap();
                    let mut found = false;
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_file() && path.extension().map_or(false, |ext| ext == "gguf") {
                                let size = entry.metadata().unwrap().len() as f64 / 1024.0 / 1024.0 / 1024.0;
                                println!("  - {:<40} [{:.2} GB]", path.file_name().unwrap().to_string_lossy(), size);
                                found = true;
                            }
                        }
                    }
                    if !found {
                        println!("No .gguf models found in {:?}", models_dir);
                    }
                }
                ModelAction::Download { name, url } => {
                    let (download_url, file_name) = if let Some(u) = url {
                        let fname = u.split('/').last().unwrap_or("model.gguf").to_string();
                        (u.clone(), fname)
                    } else {
                        match name.as_deref().unwrap_or("qwen-9b") {
                            "qwen-9b" => (
                                "https://huggingface.co/unsloth/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf".to_string(),
                                "Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf".to_string()
                            ),
                            "phi-3" => (
                                "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf".to_string(),
                                "Phi-3-mini-4k-instruct-q4.gguf".to_string()
                            ),
                            _ => {
                                println!("Unknown model preset. Available presets: qwen-9b, phi-3");
                                return;
                            }
                        }
                    };

                    let target_path = root.join("llama-cpp").join("models").join(file_name);
                    if target_path.exists() {
                        println!("Model already exists at {:?}", target_path);
                        return;
                    }

                    println!("Initializing download for {}...", target_path.display());
                    if let Err(e) = download_model(&download_url, &target_path).await {
                        println!("  \x1b[31m[ERROR]\x1b[0m Download failed: {}", e);
                    }
                }
                ModelAction::Remove { name } => {
                    let target_path = root.join("llama-cpp").join("models").join(name);
                    if target_path.exists() {
                        println!("Removing model {:?}...", target_path);
                        std::fs::remove_file(target_path).expect("Failed to remove model");
                        println!("Model removed. ✨");
                    } else {
                        println!("Model {:?} not found.", target_path);
                    }
                }
            }
        }
    }
}
