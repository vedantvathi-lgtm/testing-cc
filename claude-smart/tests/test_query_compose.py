"""Tests for the deterministic PreToolUse query composer."""

from __future__ import annotations

from claude_smart import query_compose


def test_edit_uses_basename_and_snippet() -> None:
    q = query_compose.from_tool_call(
        "Edit",
        {"file_path": "/abs/src/pkg/config.toml", "new_string": "version = '2'"},
    )
    assert "config.toml" in q
    assert "version = '2'" in q
    # Full absolute path should not leak into the query (noisy for BM25).
    assert "/abs/src/pkg/" not in q


def test_write_falls_back_to_content_when_new_string_absent() -> None:
    q = query_compose.from_tool_call(
        "Write", {"file_path": "a.py", "content": "print('hi')"}
    )
    assert "a.py" in q
    assert "print('hi')" in q


def test_bash_uses_only_first_line() -> None:
    q = query_compose.from_tool_call(
        "Bash", {"command": "git push origin main\nrm -rf node_modules"}
    )
    assert q == "git push origin main"


def test_bash_truncates_long_command() -> None:
    long = "echo " + "x" * 1000
    q = query_compose.from_tool_call("Bash", {"command": long})
    assert len(q) <= 400


def test_unknown_tool_returns_empty() -> None:
    assert query_compose.from_tool_call("Read", {"file_path": "a.py"}) == ""
    assert query_compose.from_tool_call("Glob", {"pattern": "**/*.py"}) == ""


def test_missing_fields_are_tolerated() -> None:
    assert query_compose.from_tool_call("Edit", {}) == ""
    assert query_compose.from_tool_call("Bash", {}) == ""
