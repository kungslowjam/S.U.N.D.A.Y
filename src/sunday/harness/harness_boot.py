import subprocess
import time
import os
import sys
import socket
import signal
import platform
from pathlib import Path
from typing import List, Optional, Dict, Any
from dataclasses import dataclass

# Add src to path
sys.path.append(str(Path(__file__).parent.parent.parent))


@dataclass
class ProcessInfo:
    """Information about a managed process."""
    process: subprocess.Popen
    name: str
    pid: int
    is_windows: bool = False


def is_port_in_use(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(('localhost', port)) == 0


def wait_for_backend(timeout: int = 60) -> bool:
    """Poll backend /health until ready or timeout."""
    import urllib.request
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with urllib.request.urlopen('http://127.0.0.1:8000/health', timeout=2) as r:
                if r.status == 200:
                    print('[✅] Backend is healthy.')
                    return True
        except Exception:
            pass
        time.sleep(1)
    print('[⚠️] Backend health check timed out.')
    return False


def wait_for_frontend(timeout: int = 30) -> bool:
    """Poll frontend until Vite responds with valid HTML."""
    import urllib.request
    t0 = time.time()
    while time.time() - t0 < timeout:
        # Try localhost first (Vite binds to localhost/::1 by default)
        for host in ('localhost', '127.0.0.1'):
            try:
                with urllib.request.urlopen(f'http://{host}:5173', timeout=2) as r:
                    html = r.read().decode('utf-8', errors='ignore').lower()
                    if 'vite' in html or '<div id="root"' in html or '<!doctype html>' in html:
                        print(f'[✅] Frontend is ready at http://{host}:5173.')
                        return True
            except Exception:
                pass
        time.sleep(1)
    print('[⚠️] Frontend readiness check timed out.')
    return False


class CrossPlatformProcessManager:
    """Cross-platform process management for the harness boot system.

    Handles process cleanup, startup, and termination across Windows, Linux, and macOS.
    """

    def __init__(self):
        self.system = platform.system().lower()
        self.is_windows = self.system == 'windows'
        self.managed_processes: List[ProcessInfo] = []

    def kill_process_by_name(self, name: str) -> bool:
        """Kill all processes matching the given name. Cross-platform."""
        print(f"[🧹 PRE-CLEAN] Killing processes matching '{name}'...")
        try:
            if self.is_windows:
                # Windows: use taskkill
                result = subprocess.run(
                    f"taskkill /F /IM {name} /T",
                    shell=True, capture_output=True, text=True
                )
                return result.returncode == 0 or "not found" in result.stderr.lower()
            else:
                # Linux/macOS: use pkill
                result = subprocess.run(
                    f"pkill -9 -f '{name}'",
                    shell=True, capture_output=True, text=True
                )
                return result.returncode == 0 or result.returncode == 1  # 1 = no processes found
        except Exception as e:
            print(f"      [⚠️] Failed to kill {name}: {e}")
            return False

    def cleanup_existing(self, process_names: Optional[List[str]] = None):
        """Kill lingering processes to ensure a clean cold start."""
        print("[🧹 PRE-CLEAN] Clearing old processes...")

        if process_names is None:
            if self.is_windows:
                process_names = ["llama-server.exe", "node.exe"]
            else:
                process_names = ["llama-server", "node"]

        for name in process_names:
            self.kill_process_by_name(name)

        time.sleep(1)

    def start_process(self, command: str, cwd: Path, name: str,
                      env: Optional[Dict[str, str]] = None,
                      shell: bool = True) -> ProcessInfo:
        """Start a process with cross-platform compatibility."""
        print(f"[🚀 BOOT] Starting {name}: {command}")

        # Merge current environment with new env vars
        full_env = os.environ.copy()
        if env:
            full_env.update(env)

        kwargs = {
            "cwd": cwd,
            "shell": shell,
            "env": full_env,
        }

        if self.is_windows:
            kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
        else:
            # On Unix, start a new process group for clean termination
            kwargs["start_new_session"] = True

        proc = subprocess.Popen(command, **kwargs)

        info = ProcessInfo(
            process=proc,
            name=name,
            pid=proc.pid,
            is_windows=self.is_windows
        )
        self.managed_processes.append(info)
        return info

    def terminate_process(self, info: ProcessInfo) -> bool:
        """Terminate a process gracefully, then forcefully if needed."""
        try:
            if info.process.poll() is not None:
                return True  # Already terminated

            if info.is_windows:
                # Windows: taskkill by PID
                subprocess.run(
                    f"taskkill /F /T /PID {info.pid}",
                    shell=True, capture_output=True
                )
            else:
                # Unix: try graceful SIGTERM first, then SIGKILL
                try:
                    os.killpg(os.getpgid(info.pid), signal.SIGTERM)
                    # Wait briefly for graceful shutdown
                    info.process.wait(timeout=3)
                except (ProcessLookupError, subprocess.TimeoutExpired):
                    # Force kill
                    try:
                        os.killpg(os.getpgid(info.pid), signal.SIGKILL)
                    except ProcessLookupError:
                        pass

            return True
        except Exception as e:
            print(f"      [⚠️] Failed to terminate {info.name} (PID {info.pid}): {e}")
            return False

    def terminate_all(self):
        """Terminate all managed processes."""
        if not self.managed_processes:
            return

        print("\n[🧹 CLEANUP] Shutting down managed processes...")
        for info in self.managed_processes:
            print(f"      Terminating {info.name} (PID {info.pid})...")
            self.terminate_process(info)
        print("[OK] Cleanup complete.")
        self.managed_processes.clear()

    def find_executable(self, name: str, search_paths: Optional[List[Path]] = None) -> Optional[Path]:
        """Find an executable in PATH or search paths."""
        # First check search paths
        if search_paths:
            for path in search_paths:
                candidate = path / name
                if self.is_windows:
                    candidate = candidate.with_suffix('.exe')
                if candidate.exists():
                    return candidate

        # Then check PATH
        path_var = os.environ.get('PATH', '')
        for dir_path in path_var.split(os.pathsep):
            candidate = Path(dir_path) / name
            if self.is_windows:
                candidate = candidate.with_suffix('.exe')
            if candidate.exists():
                return candidate

        return None


def load_env_file(env_path: Path) -> Dict[str, str]:
    """Load environment variables from a .env file."""
    env_vars = {}
    if env_path.exists():
        with open(env_path, "r") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" in line:
                    # Handle values that might contain =
                    key, value = line.split("=", 1)
                    env_vars[key.strip()] = value.strip().strip('"').strip("'")
        print(f"[📄] Loaded environment from {env_path}")
    return env_vars


def build_llama_command(llama_exe: Path, model_path: Path,
                        gpu_layers: int = 35, parallel_slots: int = 1) -> str:
    """Build the llama-server command with appropriate flags."""
    cmd_parts = [
        f'"{llama_exe}"',
        f'-m "{model_path}"',
        '--port 8081',
        f'-ngl {gpu_layers}',
        '-c 32768',
        '-t 4',
        f'--parallel {parallel_slots}',
        '--cache-ram 0',
        '--no-warmup'
    ]
    return ' '.join(cmd_parts)


def run_automated_harness():
    """Run the full automated cold-start harness with cross-platform support."""
    # --- Rust bridge: prefer Rust BootOrchestrator for process management ---
    try:
        import sunday_rust
        _rust_boot = sunday_rust.BootOrchestrator()
        _use_rust_boot = True
    except ImportError:
        _rust_boot = None
        _use_rust_boot = False

    pm = CrossPlatformProcessManager()
    pm.cleanup_existing()

    project_root = Path(__file__).parent.parent.parent.parent
    venv_python = project_root / ".venv" / "Scripts" / "python.exe"
    if not venv_python.exists():
        venv_python = project_root / ".venv" / "bin" / "python"

    # Find llama-server executable
    llama_exe = project_root / "llama-cpp" / "llama-server.exe"
    if not llama_exe.exists() and not pm.is_windows:
        llama_exe = project_root / "llama-cpp" / "llama-server"
        if not llama_exe.exists():
            # Try to find in PATH
            found = pm.find_executable("llama-server", [project_root / "llama-cpp"])
            if found:
                llama_exe = found

    model_path = project_root / "llama-cpp" / "models" / "Qwen3.5-9B-DeepSeek-V4-Flash-Q4_K_S.gguf"

    print("="*50)
    print("   SUNDAY AUTOMATED COLD-START HARNESS (V3)")
    print(f"   Platform: {platform.system()} {platform.release()}")
    print("="*50)

    # Load project .env if it exists
    env_vars = load_env_file(project_root / ".env")

    # 🧠 MATCH PRODUCTION ENV: Set config path and API Key
    config_path = project_root / "configs" / "sunday" / "config.toml"
    env_vars["OPENSUNDAY_CONFIG"] = str(config_path)
    if "OPENSUNDAY_API_KEY" not in env_vars:
        env_vars["OPENSUNDAY_API_KEY"] = "harness-test-key-123"

    # 🧪 HARNESS MODE: Skip SSRF checks for local testing
    env_vars["SUNDAY_HARNESS_MODE"] = "1"

    os.environ.update(env_vars)

    try:
        if _use_rust_boot:
            print("[🦀] Using Rust BootOrchestrator for cold-start...")
            _rust_boot.cold_start(8081, 8000, 5173, str(model_path) if model_path.exists() else None)
        else:
            # 1. Start AI Engine (llama-server) if not running
            if not is_port_in_use(8081):
                print("[🧠] AI Engine is down. Starting llama-server...")
                gpu_layers = int(os.environ.get("SUNDAY_GPU_LAYERS", "35"))
                parallel_slots = int(os.environ.get("SUNDAY_HARNESS_PARALLEL", "1"))

                if llama_exe.exists():
                    llama_cmd = build_llama_command(
                        llama_exe, model_path,
                        gpu_layers=gpu_layers,
                        parallel_slots=parallel_slots
                    )
                    pm.start_process(llama_cmd, project_root / "llama-cpp", "AI-ENGINE")
                    print("       Waiting for AI to warm up...")
                    time.sleep(15)  # Engines take time to load weights
                else:
                    print(f"      [⚠️] llama-server not found at {llama_exe}. Skipping AI engine startup.")
            else:
                print("[✅] AI Engine is already running.")

            # 2. Start Backend if not running
            if not is_port_in_use(8000):
                print("[⚡] Backend is down. Starting autonomous backend...")
                backend_cmd = f'"{venv_python}" -m sunday.cli serve --engine multi --agent orchestrator --host 127.0.0.1 --port 8000'
                pm.start_process(backend_cmd, project_root, "BACKEND", env=env_vars)
                wait_for_backend(timeout=60)
            else:
                print("[✅] Backend is already running.")

            # 3. Start Frontend if not running
            if not is_port_in_use(5173):
                print("[🌐] Frontend is down. Starting autonomous frontend (Vite)...")
                pm.start_process("npm run dev -- --host 127.0.0.1", project_root / "frontend", "FRONTEND")
                wait_for_frontend(timeout=30)
            else:
                print("[✅] Frontend is already running.")

        print("\n[🧪] Everything ready. Starting E2E Browser Tests...")

        try:
            from sunday.harness.runner import SkillHarness, HarnessConfig

            config = HarnessConfig(
                max_retries=2,
                max_turns=10,
                screenshot_on_pass=True,
                screenshot_on_fail=True,
            )
            harness = SkillHarness(config=config)

            # Missions to test
            missions = [
                "Navigate to the SUNDAY dashboard and verify the page title and chat interface are visible.",
                "Open the chat and send a message asking for system health check.",
            ]

            all_passed = True
            for i, m in enumerate(missions):
                print(f"\n[🔄 MISSION {i+1}] {m}")
                res = harness.run_browser_test(m, mission_idx=i+1)
                status = "✅ PASSED" if res.success else "❌ FAILED"
                print(f"      Result: {status} ({res.latency:.2f}s)")
                if not res.success:
                    print(f"      Reason: {res.output[:200]}")
                    all_passed = False
                    # Trigger healing for failed missions
                    harness.heal_tool(res)

            if all_passed:
                print("\n[🏆] All missions passed!")
            else:
                print("\n[⚠️] Some missions failed. Healing has been triggered.")

        except Exception as e:
            print(f"[❌ ERROR] Harness failed: {e}")
            import traceback
            traceback.print_exc()

    except KeyboardInterrupt:
        print("\n[🛑] Interrupted by user.")
    finally:
        pm.terminate_all()

    print("\n" + "="*50)
    print("   AUTOMATED HARNESS FINISHED")
    print("="*50)


if __name__ == "__main__":
    run_automated_harness()
