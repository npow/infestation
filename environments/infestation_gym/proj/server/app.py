from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path
from typing import Any

from openenv.core import Action, Environment, Observation, State, create_app
from pydantic import Field


LEVELS = [
    "tinderrectangle.csv",
    "cyborg_rats/ai_takeover.csv",
    "release.csv",
    "lock_in.csv",
    "reload_v3.csv",
    "chase.csv",
    "cooperation/tug_of_war.csv",
    "cooperation/handoff.csv",
    "cooperation/blocked_v2.csv",
]


class InfestationAction(Action):
    moves: str = Field(
        description=(
            "Complete candidate move string. Use ^ v < > . for one-player levels. "
            "For two-player levels, use space-separated two-character turns like '^>' or '..'."
        )
    )


class InfestationObservation(Observation):
    prompt: str
    messages: list[dict[str, str]]
    level: str
    board: str
    attempts_left: int
    last_result: str | None = None
    last_moves: str | None = None


class InfestationState(State):
    level: str | None = None
    solved: bool = False
    last_result: str | None = None


class InfestationEnv(Environment[InfestationAction, InfestationObservation, InfestationState]):
    SUPPORTS_CONCURRENT_SESSIONS = True

    def __init__(self) -> None:
        super().__init__()
        self._state = InfestationState()
        self._board = ""
        self._max_attempts = int(os.environ.get("INFESTATION_MAX_ATTEMPTS", "5"))

    @property
    def state(self) -> InfestationState:
        return self._state

    def reset(
        self,
        seed: int | None = None,
        episode_id: str | None = None,
        **_: Any,
    ) -> InfestationObservation:
        seed = 0 if seed is None else seed
        level = LEVELS[seed % len(LEVELS)]
        self._state = InfestationState(episode_id=episode_id, step_count=0, level=level)
        self._board = self._read_level(level)
        return self._observation(reward=None)

    def step(
        self,
        action: InfestationAction,
        timeout_s: float | None = None,
        **_: Any,
    ) -> InfestationObservation:
        if self._state.level is None:
            return self.reset()

        moves = action.moves.strip()
        result = self._verify(self._state.level, moves, timeout_s=timeout_s)
        self._state.step_count += 1
        self._state.last_result = result
        self._state.solved = "result=Won" in result
        done = self._state.solved or self._state.step_count >= self._max_attempts
        reward = 1.0 if self._state.solved else 0.0
        return self._observation(
            reward=reward,
            done=done,
            last_moves=moves,
            last_result=result,
        )

    def _observation(
        self,
        reward: float | None,
        done: bool = False,
        last_moves: str | None = None,
        last_result: str | None = None,
    ) -> InfestationObservation:
        assert self._state.level is not None
        attempts_left = max(0, self._max_attempts - self._state.step_count)
        prompt = self._prompt(
            level=self._state.level,
            board=self._board,
            attempts_left=attempts_left,
            last_moves=last_moves,
            last_result=last_result,
        )
        return InfestationObservation(
            prompt=prompt,
            messages=[{"role": "user", "content": prompt}],
            level=self._state.level,
            board=self._board,
            attempts_left=attempts_left,
            last_result=last_result,
            last_moves=last_moves,
            reward=reward,
            done=done,
        )

    def _prompt(
        self,
        level: str,
        board: str,
        attempts_left: int,
        last_moves: str | None,
        last_result: str | None,
    ) -> str:
        feedback = ""
        if last_result is not None:
            feedback = f"\nPrevious submission: `{last_moves}`\nOracle result: `{last_result}`\n"
        return (
            "Solve this Infestation level. Submit only a move string, or a JSON object "
            'like {"moves":"..."}.\n\n'
            "Move alphabet: ^ up, v down, < left, > right, . stall. "
            "For two-player boards, each turn is two characters, player 1 then player 2, "
            "with turns separated by spaces, for example `^^ <> v.`.\n"
            "The reward is 1 only if the real game oracle reports result=Won. "
            f"You have {attempts_left} attempt(s) left in this episode."
            f"{feedback}\nLevel: {level}\nBoard CSV:\n{board}\n"
        )

    def _verify(self, level: str, moves: str, timeout_s: float | None) -> str:
        solver = _solver_path()
        level_path = _levels_dir() / level
        timeout = timeout_s if timeout_s is not None else 15.0
        try:
            completed = subprocess.run(
                [str(solver), "verify", str(level_path), moves],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired:
            return f"oracle timeout after {timeout:g}s"
        output = completed.stdout.strip()
        return _compact_output(output)

    def _read_level(self, level: str) -> str:
        return (_levels_dir() / level).read_text().strip()


def _solver_path() -> Path:
    configured = os.environ.get("INFESTATION_SOLVER")
    if configured:
        return Path(configured)

    repo_root = Path(__file__).resolve().parents[4]
    local_solver = repo_root / "target" / "release" / "solver"
    if local_solver.exists():
        return local_solver
    return Path("/app/infestation/solver")


def _levels_dir() -> Path:
    configured = os.environ.get("INFESTATION_LEVELS_DIR")
    if configured:
        return Path(configured)

    repo_root = Path(__file__).resolve().parents[4]
    local_levels = repo_root / "levels"
    if local_levels.exists():
        return local_levels
    return Path("/app/infestation/levels")


def _compact_output(output: str) -> str:
    output = re.sub(r"\s+", " ", output).strip()
    return output if output else "no oracle output"


app = create_app(
    InfestationEnv,
    InfestationAction,
    InfestationObservation,
    env_name="infestation-gym",
)
