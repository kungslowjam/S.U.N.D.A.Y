"""Command-line interface for SUNDAY (Click-based)."""

from __future__ import annotations

import click

import sunday
from sunday.cli._bootstrap import bootstrap_cmd
from sunday.cli.add_cmd import add
from sunday.cli.agent_cmd import agent
from sunday.cli.ask import ask
from sunday.cli.bench_cmd import bench
from sunday.cli.channel_cmd import channel
from sunday.cli.channels_cmd import channels
from sunday.cli.chat_cmd import chat
from sunday.cli.compose_cmd import compose
from sunday.cli.config_cmd import config
from sunday.cli.connect_cmd import connect
from sunday.cli.daemon_cmd import restart, start, status, stop
from sunday.cli.deep_research_setup_cmd import deep_research_setup
from sunday.cli.digest_cmd import digest
from sunday.cli.doctor_cmd import doctor
from sunday.cli.eval_cmd import eval_group
from sunday.cli.feedback_cmd import feedback_group
from sunday.cli.gateway_cmd import gateway
from sunday.cli.host_cmd import host
from sunday.cli.init_cmd import init
from sunday.cli.memory_cmd import memory
from sunday.cli.mine_cmd import mine
from sunday.cli.model import model
from sunday.cli.operators_cmd import operators
from sunday.cli.optimize_cmd import optimize_group
from sunday.cli.pearl_cmd import pearl
from sunday.cli.quickstart_cmd import quickstart
from sunday.cli.registry_cmd import registry
from sunday.cli.scan_cmd import scan
from sunday.cli.scheduler_cmd import scheduler
from sunday.cli.serve import serve
from sunday.cli.skill_cmd import skill
from sunday.cli.telemetry_cmd import telemetry
from sunday.cli.tool_cmd import tool
from sunday.cli.vault_cmd import vault
from sunday.cli.workflow_cmd import workflow
from sunday.learning.distillation.cli import learning_group


@click.group(
    help="SUNDAY — modular AI assistant backend",
    invoke_without_command=True,
)
@click.version_option(version=sunday.__version__, prog_name="sunday")
@click.option("--verbose", is_flag=True, default=False, help="Enable debug logging")
@click.option("--quiet", is_flag=True, default=False, help="Suppress non-error output")
@click.pass_context
def cli(ctx: click.Context, verbose: bool, quiet: bool) -> None:
    """Top-level CLI group."""
    from sunday.cli.log_config import setup_logging

    ctx.ensure_object(dict)
    ctx.obj["verbose"] = verbose
    ctx.obj["quiet"] = quiet
    setup_logging(verbose=verbose, quiet=quiet)

    # Check for updates on interactive commands
    if not quiet and ctx.invoked_subcommand:
        from sunday.cli._version_check import check_for_updates

        check_for_updates(ctx.invoked_subcommand)

    # First-run guard — routes bare `sunday` to chat or init.
    if ctx.invoked_subcommand is None:
        from sunday.cli._first_run import check_and_route

        check_and_route(ctx)


cli.add_command(init, "init")
cli.add_command(ask, "ask")
cli.add_command(chat, "chat")
cli.add_command(serve, "serve")
cli.add_command(model, "model")
cli.add_command(memory, "memory")
cli.add_command(mine, "mine")
cli.add_command(pearl, "pearl")
cli.add_command(telemetry, "telemetry")
cli.add_command(bench, "bench")
cli.add_command(channel, "channel")
cli.add_command(channels, "channels")
cli.add_command(scheduler, "scheduler")
cli.add_command(doctor, "doctor")
cli.add_command(agent, "agents")
cli.add_command(workflow, "workflow")
cli.add_command(skill, "skill")
cli.add_command(start, "start")
cli.add_command(stop, "stop")
cli.add_command(restart, "restart")
cli.add_command(status, "status")
cli.add_command(vault, "vault")
cli.add_command(add, "add")
cli.add_command(operators, "operators")
cli.add_command(eval_group, "eval")
cli.add_command(host, "host")
cli.add_command(quickstart, "quickstart")
cli.add_command(optimize_group, "optimize")
cli.add_command(feedback_group, "feedback")
cli.add_command(compose, "compose")
cli.add_command(gateway, "gateway")
cli.add_command(tool, "tool")
cli.add_command(registry, "registry")
cli.add_command(config, "config")
cli.add_command(scan, "scan")
cli.add_command(connect, "connect")
cli.add_command(digest, "digest")
cli.add_command(deep_research_setup, "deep-research-setup")
cli.add_command(deep_research_setup, "research")
cli.add_command(learning_group, "learning")
cli.add_command(bootstrap_cmd, "_bootstrap")

# Gateway CLI commands (lazy import to avoid pulling starlette)
try:
    from sunday.cli.auth_cmd import auth

    cli.add_command(auth, "auth")
except ImportError:
    pass

try:
    from sunday.cli.tunnel_cmd import tunnel

    cli.add_command(tunnel, "tunnel")
except ImportError:
    pass


def main() -> None:
    """Entry point registered as ``sunday`` console script."""
    cli()


__all__ = ["cli", "main"]
