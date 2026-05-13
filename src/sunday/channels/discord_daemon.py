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
    from pathlib import Path

    # Write PID
    pid_path = Path(_PID_FILE)
    pid_path.parent.mkdir(parents=True, exist_ok=True)
    pid_path.write_text(str(os.getpid()))

    from sunday.agents.orchestrator import OrchestratorAgent
    from sunday.core.registry import ToolRegistry

    # Build dependencies for knowledge tools
    from sunday.core.config import load_config, DEFAULT_CONFIG_DIR
    from sunday.connectors.store import KnowledgeStore
    from sunday.connectors.retriever import TwoStageRetriever
    from sunday.engine import discover_engines
    from sunday.engine.multi import MultiEngine

    config = load_config()
    all_engines = discover_engines(config)
    engine = MultiEngine(all_engines)
    
    knowledge_db_path = str(DEFAULT_CONFIG_DIR / "knowledge.db")
    store = None
    retriever = None
    if Path(knowledge_db_path).exists():
        try:
            store = KnowledgeStore(knowledge_db_path)
            retriever = TwoStageRetriever(store)
        except Exception as e:
            logger.warning(f"Failed to initialize knowledge store: {e}")

    # Build ALL tools from registry
    tools = []
    # Explicitly import all tool modules to populate registry
    import sunday.tools
    from sunday.server.agent_manager_routes import _ensure_registries_populated
    _ensure_registries_populated()

    for tool_name in ToolRegistry.keys():
        try:
            tool_cls = ToolRegistry.get(tool_name)
            
            # Instantiate with dependencies if needed
            if tool_name in ("knowledge_search", "retrieval"):
                if retriever:
                    tools.append(tool_cls(retriever=retriever))
            elif tool_name in ("knowledge_sql", "scan_chunks"):
                if store:
                    if tool_name == "scan_chunks":
                        tools.append(tool_cls(store=store, engine=engine, model=model))
                    else:
                        tools.append(tool_cls(store=store))
            elif tool_name == "think":
                tools.append(tool_cls())
            else:
                # Try default instantiation
                tools.append(tool_cls())
        except Exception as e:
            logger.debug(f"Skipping tool {tool_name} due to instantiation error: {e}")

    agent = OrchestratorAgent(
        engine=engine,
        model=model,
        tools=tools,
        max_turns=10,
        system_prompt=(
            "You are SUNDAY, a powerful personal AI assistant. "
            "You have access to the user's private knowledge base, the web, "
            "a web browser, and various system tools. "
            "Help the user with any request by planning and using the best tools available. "
            "Be conversational, concise, and helpful."
        )
    )
    logger.info(f"Discord daemon: Orchestrator ready with {len(tools)} tools")

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
                final_content = result.content or "No results found."
                
                # Extract observability info
                used_tools = sorted(list(set(tr.tool_name for tr in result.tool_results)))
                sources = result.metadata.get("sources", [])
                
                # Build footer
                footer_parts = []
                if used_tools:
                    footer_parts.append(f"🛠️ **Tools**: {', '.join(used_tools)}")
                if sources:
                    footer_parts.append(f"📚 **Sources**: {len(sources)} cited")
                if result.turns > 1:
                    footer_parts.append(f"🔄 **Turns**: {result.turns}")
                
                if footer_parts:
                    final_content += "\n\n" + " | ".join(footer_parts)
                
                return final_content
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
