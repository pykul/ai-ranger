"""Real agent subprocess management for integration tests."""

import asyncio
import json
import os
import signal


def is_root() -> bool:
    """Return True if running with elevated privileges (root on Unix, Administrator on Windows)."""
    if os.name == "nt":
        import ctypes
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    return os.getuid() == 0


class AgentProcess:
    """Manages a real agent subprocess for integration testing."""

    def __init__(self, binary_path: str):
        self._binary = binary_path
        self._proc: asyncio.subprocess.Process | None = None
        self._stdout_lines: list[str] = []
        self._reader_task: asyncio.Task | None = None

    async def start(
        self,
        agent_id: str | None = None,
        backend_url: str | None = None,
        token: str | None = None,
        config_path: str | None = None,
    ) -> None:
        """Start the agent as a subprocess.

        If token and backend_url are provided, passes --token and --backend
        for auto-enrollment. Otherwise runs in standalone stdout mode.
        """
        cmd = [self._binary]
        if token and backend_url:
            cmd.extend(["--token", token, "--backend", backend_url])
        if config_path:
            cmd.extend(["--config", config_path])

        self._proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        # Read stdout in background
        self._reader_task = asyncio.create_task(self._read_stdout())

        # Wait for agent to start capturing
        for _ in range(30):
            await asyncio.sleep(0.5)
            if any("Capturing" in line or "provider" in line for line in self._stdout_lines):
                break

    async def _read_stdout(self) -> None:
        """Read stdout lines into buffer."""
        assert self._proc and self._proc.stdout
        while True:
            line = await self._proc.stdout.readline()
            if not line:
                break
            decoded = line.decode().strip()
            if decoded:
                self._stdout_lines.append(decoded)

    async def stop(self) -> None:
        """Send SIGTERM and wait for clean exit."""
        if self._proc and self._proc.returncode is None:
            self._proc.send_signal(signal.SIGTERM)
            try:
                await asyncio.wait_for(self._proc.wait(), timeout=10)
            except asyncio.TimeoutError:
                self._proc.kill()
                await self._proc.wait()
        if self._reader_task:
            self._reader_task.cancel()
            try:
                await self._reader_task
            except asyncio.CancelledError:
                pass

    def get_events(self) -> list[dict]:
        """Return parsed JSON events from agent stdout."""
        events = []
        for line in self._stdout_lines:
            try:
                obj = json.loads(line)
                if "provider" in obj:
                    events.append(obj)
            except json.JSONDecodeError:
                continue
        return events

    def get_stderr_lines(self) -> list[str]:
        """Return stderr lines (log messages)."""
        return [line for line in self._stdout_lines if not line.startswith("{")]
