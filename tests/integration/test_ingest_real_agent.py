"""Real agent binary integration tests. Requires root/Administrator for raw socket capture."""

import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request

import pytest

from helpers.agent import is_root

IS_WINDOWS = sys.platform == "win32"

pytestmark = pytest.mark.skipif(
    not is_root(),
    reason="Requires root/Administrator for raw socket capture",
)


def _stop_agent(proc):
    """Stop the agent process. Uses SIGTERM on Unix, terminate() on Windows."""
    if IS_WINDOWS:
        proc.terminate()
    else:
        proc.send_signal(signal.SIGTERM)


def _trigger_ai_traffic():
    """Make an HTTPS request to an AI provider to generate a capturable event."""
    try:
        urllib.request.urlopen("https://api.openai.com", timeout=10)
    except Exception:
        pass


def test_real_agent_enrollment(agent_binary):
    """The agent binary successfully enrolls against the local gateway."""
    from conftest import GATEWAY_URL, SEED_TOKEN

    result = subprocess.run(
        [agent_binary, "--enroll", "--token", SEED_TOKEN, "--backend", GATEWAY_URL],
        capture_output=True, text=True, timeout=15,
    )
    assert result.returncode == 0, f"Enrollment failed: {result.stderr}"
    assert "Enrolled as" in result.stderr


@pytest.mark.network
def test_real_agent_captures_sni(agent_binary):
    """Start agent in stdout mode, trigger AI provider traffic, verify JSON event."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
        f.write('[agent]\nmode = "dns-sni"\n\n[[outputs]]\ntype = "stdout"\n')
        config_path = f.name

    try:
        proc = subprocess.Popen(
            [agent_binary, "--config", config_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        time.sleep(3)

        _trigger_ai_traffic()
        time.sleep(3)

        _stop_agent(proc)
        stdout, stderr = proc.communicate(timeout=10)

        events = []
        for line in stdout.decode().splitlines():
            try:
                obj = json.loads(line)
                if "provider" in obj:
                    events.append(obj)
            except json.JSONDecodeError:
                continue

        assert len(events) > 0, f"No events captured. stderr: {stderr.decode()[:500]}"
        assert any(e.get("provider") == "openai" for e in events)

    finally:
        os.unlink(config_path)
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def test_real_agent_stdout_mode(agent_binary):
    """Agent in stdout mode captures and prints JSON events without backend."""
    proc = subprocess.Popen(
        [agent_binary],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        time.sleep(3)

        _trigger_ai_traffic()
        time.sleep(3)

        _stop_agent(proc)
        stdout, _ = proc.communicate(timeout=10)

        events = []
        for line in stdout.decode().splitlines():
            try:
                obj = json.loads(line)
                if "provider" in obj:
                    events.append(obj)
            except json.JSONDecodeError:
                continue

        assert len(events) > 0, "No events captured in stdout mode"
        assert "provider" in events[0]

    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
