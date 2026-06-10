from pathlib import Path
from typing import Any

import verifiers as vf


def render_prompt(observation: Any) -> list[dict[str, Any]]:
    if isinstance(observation, dict):
        messages = observation.get("messages")
        if isinstance(messages, list) and messages:
            return messages
        prompt = observation.get("prompt")
        if isinstance(prompt, str) and prompt.strip():
            return [{"role": "user", "content": prompt}]
    raise RuntimeError(
        "OpenEnv observation did not include a renderable prompt. "
        "Update render_prompt() for your project's observation schema."
    )


def load_environment(
    num_train_examples: int = 100,
    num_eval_examples: int = 50,
    seed: int = 0,
):
    package_root = Path(__file__).resolve().parents[1]
    return vf.OpenEnvEnv(
        openenv_project=package_root / "proj",
        num_train_examples=num_train_examples,
        num_eval_examples=num_eval_examples,
        seed=seed,
        prompt_renderer=render_prompt,
        max_turns=5,
    )
