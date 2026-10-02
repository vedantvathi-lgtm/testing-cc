"""LLM judge for scoring retrieval quality against ground truth.

Runs the Claude CLI (user's OAuth login, not ``--bare`` which requires an
API key) from an isolated ``/tmp/membench-judge/`` scratch dir so the
judge's own plugin-captured memory lands in a project we never query —
the benchmark is scoped per-scenario-project, so judge sessions can't
contaminate scenario retrievals.
"""

from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path

_JUDGE_MODEL = "claude-haiku-4-5"
_JUDGE_TIMEOUT_S = 90
_JUDGE_CWD = Path("/tmp/membench-judge")

_SYSTEM_PROMPT = (
    "You are a strict evaluator comparing memory-system retrievals against "
    "a written ground truth. You return JSON only, no prose."
)

_SCORE_SCHEMA = {
    "type": "object",
    "properties": {
        "recall": {
            "type": "integer",
            "minimum": 0,
            "maximum": 3,
            "description": "3=ground truth clearly present; 2=present but vague; 1=tangential; 0=absent or contradicted",
        },
        "specificity": {
            "type": "integer",
            "minimum": 0,
            "maximum": 3,
            "description": "3=precise enough to act on; 2=mostly specific; 1=generic; 0=no actionable detail",
        },
        "rationale": {
            "type": "string",
            "description": "Under 20 words explaining the score.",
        },
    },
    "required": ["recall", "specificity", "rationale"],
    "additionalProperties": False,
}


@dataclass
class JudgeScore:
    """Structured score from the judge.

    Attributes:
        recall (int): 0–3; did the ground truth survive extraction?
        specificity (int): 0–3; is the retrieved text precise enough to act on?
        rationale (str): Short explanation.
        error (str | None): Non-None if the judge call failed; both scores
            are 0 in that case.
    """

    recall: int
    specificity: int
    rationale: str
    error: str | None = None


def _build_prompt(*, ground_truth: str, probe: str, retrieved: list[str]) -> str:
    """Render the judge prompt from scenario + retrieval.

    Args:
        ground_truth (str): The fact the system should have remembered.
        probe (str): The probe query that was fired at the system.
        retrieved (list[str]): Lines returned by the system's adapter.

    Returns:
        str: The user message for the judge.
    """
    if retrieved:
        retrieved_block = "\n".join(f"- {line}" for line in retrieved)
    else:
        retrieved_block = "(no rows returned)"
    return (
        f"GROUND TRUTH:\n{ground_truth}\n\n"
        f"PROBE QUERY:\n{probe}\n\n"
        f"RETRIEVED MEMORY:\n{retrieved_block}\n\n"
        "Score the retrieval on 'recall' (is the ground truth present?) and "
        "'specificity' (is it precise enough to act on?). Return JSON only."
    )


def score(*, ground_truth: str, probe: str, retrieved: list[str]) -> JudgeScore:
    """Run the judge on one retrieval and return a structured score.

    Args:
        ground_truth (str): Scenario's ground-truth statement.
        probe (str): Scenario's probe query.
        retrieved (list[str]): Strings returned by a system adapter.

    Returns:
        JudgeScore: ``error`` is set if the judge call or parse failed.
    """
    prompt = _build_prompt(ground_truth=ground_truth, probe=probe, retrieved=retrieved)
    _JUDGE_CWD.mkdir(parents=True, exist_ok=True)
    cmd = [
        "claude",
        "--print",
        "--output-format",
        "json",
        "--model",
        _JUDGE_MODEL,
        "--append-system-prompt",
        _SYSTEM_PROMPT,
        "--json-schema",
        json.dumps(_SCORE_SCHEMA),
        prompt,
    ]
    env = dict(os.environ)
    env.pop("CLAUDE_SMART_STATE_DIR", None)
    try:
        proc = subprocess.run(
            cmd,
            cwd=_JUDGE_CWD,
            env=env,
            capture_output=True,
            text=True,
            timeout=_JUDGE_TIMEOUT_S,
            check=False,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired) as exc:
        return JudgeScore(0, 0, "", error=f"judge subprocess: {exc}")

    if proc.returncode != 0:
        return JudgeScore(
            0, 0, "", error=f"exit {proc.returncode}: {proc.stderr[-200:]}"
        )

    raw = proc.stdout.strip()
    try:
        envelope = json.loads(raw)
    except json.JSONDecodeError as exc:
        return JudgeScore(
            0, 0, "", error=f"judge envelope parse: {exc}; raw={raw[:200]}"
        )

    payload = envelope.get("structured_output")
    if not isinstance(payload, dict):
        # Fallback: try to parse the free-text result as JSON.
        result_text = envelope.get("result", "")
        start = result_text.find("{")
        end = result_text.rfind("}")
        if start == -1 or end == -1:
            return JudgeScore(
                0, 0, "", error=f"no structured_output; result={result_text[:200]}"
            )
        try:
            payload = json.loads(result_text[start : end + 1])
        except json.JSONDecodeError as exc:
            return JudgeScore(0, 0, "", error=f"judge result parse: {exc}")

    try:
        return JudgeScore(
            recall=int(payload["recall"]),
            specificity=int(payload["specificity"]),
            rationale=str(payload.get("rationale", ""))[:200],
        )
    except (KeyError, ValueError, TypeError) as exc:
        return JudgeScore(
            0, 0, "", error=f"judge payload shape: {exc}; raw={raw[:200]}"
        )
