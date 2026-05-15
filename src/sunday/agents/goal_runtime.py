"""GoalRuntime — autonomous loop controller for persistent objectives."""

from __future__ import annotations

import logging
import time
from typing import Any, Dict, List, Optional

from sunday.core.events import EventBus, EventType
from sunday.core.types import Goal, GoalStatus, Message, Role

logger = logging.getLogger(__name__)


class GoalRuntime:
    """Manages the lifecycle of persistent goals and automatic turn continuation.

    This implements the 'Stop Hook' pattern: if a goal is active, the runtime
    automatically triggers the next agent turn until the goal is marked complete.
    """

    def __init__(
        self,
        agent: Any,
        bus: Optional[EventBus] = None,
        max_iterations: int = 50,
    ):
        self._agent = agent
        self._bus = bus
        self._max_iterations = max_iterations
        self._active_goal: Optional[Goal] = None

    def start_goal(self, objective: str, token_budget: Optional[int] = None) -> Goal:
        """Initialize a new persistent goal."""
        self._active_goal = Goal(
            objective=objective,
            token_budget=token_budget,
            status=GoalStatus.ACTIVE,
        )
        if self._bus:
            self._bus.publish(EventType.GOAL_STARTED, {"goal": self._active_goal})
        return self._active_goal

    def run_until_complete(self, user_query: str) -> Dict[str, Any]:
        """Execute the autonomous loop for the active goal."""
        if not self._active_goal:
            raise ValueError("No active goal set. Call start_goal() first.")

        iterations = 0
        history: List[Message] = [
            Message(role=Role.USER, content=user_query)
        ]

        logger.info(f"Goal started: {self._active_goal.objective}")

        while self._active_goal.status == GoalStatus.ACTIVE and iterations < self._max_iterations:
            iterations += 1
            logger.info(f"Goal iteration {iterations}/{self._max_iterations}")

            # Inject goal context into system prompt if needed
            # (Or let the agent handle it via its own knowledge of the goal)
            
            result = self._agent.run(history[-1].content)
            
            # Update goal stats
            self._active_goal.tokens_used += result.metadata.get("tokens", 0)
            self._active_goal.time_used_seconds += result.metadata.get("duration", 0)
            self._active_goal.updated_at = time.time()

            # Check for completion (Agent should call a tool or return a specific string)
            # In SUNDAY, we look for 'update_goal(status="complete")' in tool results
            # or a metadata flag.
            
            is_complete = False
            for tr in result.tool_results:
                if tr.tool_name == "update_goal" and tr.metadata.get("status") == "complete":
                    is_complete = True
                    break
                # Special case for coding: if tests pass and agent says it's done
                if tr.tool_name == "shell_exec" and tr.success and "all tests passed" in tr.content.lower():
                    # We still want the agent to explicitly say it's done, but this is a signal
                    pass

            if is_complete or "GOAL_COMPLETE" in result.content:
                self._active_goal.status = GoalStatus.COMPLETE
                logger.info("Goal achieved!")
            elif iterations >= self._max_iterations:
                self._active_goal.status = GoalStatus.FAILED
                logger.warning("Goal failed: Max iterations reached.")
            elif self._active_goal.token_budget and self._active_goal.tokens_used > self._active_goal.token_budget:
                self._active_goal.status = GoalStatus.BUDGET_LIMITED
                logger.warning("Goal paused: Token budget exceeded.")
            else:
                # Automagically continue! 
                # We feed the assistant's response back as a message and prompt for the next step.
                history.append(Message(role=Role.ASSISTANT, content=result.content))
                # The 'Stop Hook' prompt:
                history.append(Message(role=Role.USER, content="Continue working toward the goal. If there are errors, fix them. If you are finished, report completion."))

        return {
            "goal": self._active_goal,
            "iterations": iterations,
            "final_content": result.content,
        }

__all__ = ["GoalRuntime"]
