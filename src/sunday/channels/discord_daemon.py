"""Discord daemon — listens for DMs and mentions and responds with DeepResearch.

Run as a standalone process or import start_discord_daemon() to spawn
from the server. Uses discord.py for gateway connection.
"""

from __future__ import annotations

import logging
import os
import signal
import sys
from pathlib import Path
from typing import Any

_LOG_FILE = str(Path.home() / ".sunday" / "discord-daemon.log")
Path(_LOG_FILE).parent.mkdir(parents=True, exist_ok=True)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s %(message)s",
    handlers=[
        logging.FileHandler(_LOG_FILE),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

_PID_FILE = str(Path.home() / ".sunday" / "discord-daemon.pid")

def run_discord_daemon(
    bot_token: str,
    model: str = "qwen3.5:9b",
) -> None:
    import asyncio
    import discord
    from sunday.agents.deep_research import DeepResearchAgent
    from sunday.engine.ollama import OllamaEngine
    from sunday.server.agent_manager_routes import _build_deep_research_tools

    # Write PID
    pid_path = Path(_PID_FILE)
    pid_path.parent.mkdir(parents=True, exist_ok=True)
    pid_path.write_text(str(os.getpid()))

    # Build engine using SUNDAY's discovery logic
    from sunday.core.config import load_config
    from sunday.engine import discover_engines
    from sunday.engine.multi import MultiEngine
    
    config = load_config()
    all_engines = discover_engines(config)
    
    # Add llamacpp explicitly if it's missing but we have a .gguf model
    found_llamacpp = any(ek == "llamacpp" for ek, _ in all_engines)
    if not found_llamacpp and model.endswith(".gguf"):
        from sunday.engine.openai_compat_engines import LlamaCppEngine
        host = getattr(config.engine, "llamacpp_host", "http://localhost:8081")
        all_engines.append(("llamacpp", LlamaCppEngine(host=host)))
        
    engine = MultiEngine(all_engines)
    
    tools = _build_deep_research_tools(engine=engine, model=model)
    agent = DeepResearchAgent(
        engine=engine,
        model=model,
        tools=tools,
        max_turns=5,
    )
    logger.info(f"Discord daemon: agent ready using MultiEngine with {len(all_engines)} backends")

    intents = discord.Intents.default()
    intents.message_content = True
    client = discord.Client(intents=intents)

    @client.event
    async def on_ready():
        logger.info(f"Discord daemon: Logged in as {client.user}")

    @client.event
    async def on_message(message):
        if message.author == client.user:
            return
        
        # Process all messages the bot can see (DMs and all Server Channels)
        # We already check for author == client.user above to prevent loops
        
        text = message.content
        if client.user in message.mentions:
            # Still clean up the mention if it exists
            text = text.replace(f"<@{client.user.id}>", "").strip()
            text = text.replace(f"<@!{client.user.id}>", "").strip()

        if not text:
            return

        logger.info(f"Discord Message: {text[:60]}")
        
        # Send initial response
        await message.channel.send("Message received! Working on it now...")

        # Run agent in thread
        def _run_agent():
            try:
                result = agent.run(text)
                return result.content or "No results found."
            except Exception as exc:
                logger.error(f"Discord daemon error: {exc}")
                return f"Error: {exc}"

        loop = asyncio.get_event_loop()
        reply = await loop.run_in_executor(None, _run_agent)

        # Discord has a 2000 character limit
        if len(reply) > 1950:
            reply = reply[:1950] + "\n\n(truncated)"
            
        await message.channel.send(reply)
        logger.info("Discord reply sent")

    async def _start():
        try:
            await client.start(bot_token)
        except Exception as e:
            logger.error(f"Discord client failed: {e}")
            if pid_path.exists():
                pid_path.unlink()
            sys.exit(1)

    # Graceful shutdown
    def _stop(signum: int, frame: Any) -> None:
        logger.info("Discord daemon stopping...")
        # Since we use asyncio.run(_start()), we should let it exit
        if pid_path.exists():
            pid_path.unlink()
        sys.exit(0)

    signal.signal(signal.SIGTERM, _stop)
    signal.signal(signal.SIGINT, _stop)

    asyncio.run(_start())

def start_discord_daemon(
    bot_token: str,
    model: str = "qwen3.5:9b",
) -> int:
    import subprocess
    import os
    
    # Get the project root (where src/ is)
    # This file is in src/sunday/channels/discord_daemon.py
    current_file = os.path.abspath(__file__)
    project_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(current_file))))
    src_dir = os.path.join(project_root, "src")

    env = os.environ.copy()
    env["PYTHONPATH"] = src_dir + os.pathsep + env.get("PYTHONPATH", "")

    proc = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "sunday.channels.discord_daemon",
            "--bot-token",
            bot_token,
            "--model",
            model,
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=project_root,
        env=env,
        start_new_session=True,
    )
    return proc.pid

def stop_daemon() -> bool:
    pid_path = Path(_PID_FILE)
    if not pid_path.exists():
        return False
    try:
        pid = int(pid_path.read_text().strip())
        os.kill(pid, signal.SIGTERM)
        pid_path.unlink(missing_ok=True)
        return True
    except (ValueError, ProcessLookupError, PermissionError):
        pid_path.unlink(missing_ok=True)
        return False

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--bot-token", required=False)
    parser.add_argument("--model", default="qwen3.5:9b")
    parser.add_argument("--stop", action="store_true")
    args = parser.parse_args()

    if args.stop:
        if stop_daemon():
            print("Discord daemon stopped.")
        else:
            print("Discord daemon not running.")
        sys.exit(0)

    if not args.bot_token:
        parser.error("--bot-token is required unless --stop is used")

    run_discord_daemon(bot_token=args.bot_token, model=args.model)
