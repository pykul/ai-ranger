"""Poll-with-timeout helper for integration tests. Never use time.sleep() in tests."""

import time
from typing import Callable


def wait_for_condition(
    condition: Callable[[], bool],
    timeout_secs: int = 15,
    poll_interval_secs: float = 0.5,
    description: str = "condition",
) -> None:
    """Poll condition() until True or timeout.

    Raises AssertionError with a descriptive message on timeout.
    """
    deadline = time.monotonic() + timeout_secs
    while time.monotonic() < deadline:
        if condition():
            return
        time.sleep(poll_interval_secs)
    raise AssertionError(f"Timed out waiting for {description} after {timeout_secs}s")


def wait_for_clickhouse_event(
    client,
    agent_id: str,
    provider: str,
    timeout_secs: int = 15,
) -> dict:
    """Wait for a specific event to appear in ClickHouse ai_events table.

    Returns the first matching row as a dict.
    """
    query = (
        "SELECT * FROM ai_events "
        "WHERE toString(agent_id) = %(agent_id)s AND provider = %(provider)s "
        "ORDER BY timestamp DESC LIMIT 1"
    )
    params = {"agent_id": agent_id, "provider": provider}

    deadline = time.monotonic() + timeout_secs
    while time.monotonic() < deadline:
        result = client.query(query, parameters=params)
        if result.result_rows:
            cols = result.column_names
            row = result.result_rows[0]
            return dict(zip(cols, row))
        time.sleep(0.5)

    raise AssertionError(
        f"No ClickHouse event for agent_id={agent_id} provider={provider} after {timeout_secs}s"
    )
