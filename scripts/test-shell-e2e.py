#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import base64
import errno
import fcntl
import json
import os
import pty
import struct
import tempfile
import termios
from pathlib import Path
from typing import Any

from aiohttp import WSMsgType, web


SUBPROTOCOL = "d1v-terminal.v1"
TICKET_PREFIX = "e2e-shell-ticket-"


def jwt(payload: dict[str, Any]) -> str:
    def encode(value: dict[str, Any]) -> str:
        raw = json.dumps(value, separators=(",", ":")).encode()
        return base64.urlsafe_b64encode(raw).decode().rstrip("=")

    return f"{encode({'alg': 'none'})}.{encode(payload)}.signature"


VALID_TOKEN = jwt({"sub": "shell-e2e", "exp": 9_999_999_999})
EXPIRED_TOKEN = jwt({"sub": "shell-e2e", "exp": 1})


def response(data: Any) -> web.Response:
    return web.json_response({"code": 0, "msg": "success", "data": data})


def session_payload(
    record: dict[str, Any], *, status: str = "terminated"
) -> dict[str, Any]:
    return {
        "session_id": record["session_id"],
        "workspace_scope": record["workspace_scope"],
        "project_id": record["project_id"],
        "runtime_provider": "e2e",
        "node_id": "node-shell-e2e",
        "cwd": record["cwd"],
        "mode": record["body"]["mode"],
        "status": status,
        "created_at": "2026-08-22T12:00:00Z",
        "connected_at": "2026-08-22T12:00:01Z",
        "last_seen_at": "2026-08-22T12:00:02Z",
        "ended_at": "2026-08-22T12:00:03Z" if status == "terminated" else None,
        "exit_code": 0 if status == "terminated" else None,
        "termination_reason": "client_close" if status == "terminated" else None,
        "bytes_in": 1,
        "bytes_out": 1,
    }


def require_control_headers(request: web.Request) -> None:
    assert request.headers.get("Authorization") == f"Bearer {VALID_TOKEN}"
    assert request.headers.get("X-D1V-Client") == "d1v-cli"


async def current_user(request: web.Request) -> web.Response:
    assert request.headers.get("X-D1V-Client") == "d1v-cli"
    if request.headers.get("Authorization") == f"Bearer {EXPIRED_TOKEN}":
        return web.json_response(
            {"code": 401, "msg": "unauthorized", "data": None}, status=401
        )
    require_control_headers(request)
    return response(
        {
            "id": 7,
            "is_agent": False,
            "picture": "",
            "email": "shell-e2e@example.com",
            "last_login_type": None,
            "stripe_customer_id": None,
        }
    )


async def create_session(request: web.Request) -> web.Response:
    require_control_headers(request)
    state = request.app["state"]
    body = await request.json()
    session_id = f"sh_e2e_{len(state['sessions']) + 1}"
    project_id = request.match_info.get("project_id")
    organization_id = request.query.get("organization_id")
    if project_id:
        assert body["target"] == "project"
        cwd = f"/workspace-root/projects/{project_id}"
        workspace_scope = "user:7"
    elif organization_id:
        assert body["target"] == "workspace"
        cwd = "/workspace-root"
        workspace_scope = f"organization:{organization_id}"
    else:
        assert body["target"] == "workspace"
        cwd = "/workspace-root"
        workspace_scope = "user:7"
    record = {
        "session_id": session_id,
        "body": body,
        "project_id": project_id,
        "organization_id": organization_id,
        "workspace_scope": workspace_scope,
        "cwd": cwd,
        "ticket": f"{TICKET_PREFIX}{session_id}",
        "controls": [],
        "inputs": [],
        "closed": False,
    }
    state["sessions"][session_id] = record
    return response(
        {
            "session_id": session_id,
            "workspace_scope": workspace_scope,
            "project_id": project_id,
            "runtime_provider": "e2e",
            "node_id": "node-shell-e2e",
            "cwd": cwd,
            "transport": "direct",
            "websocket_url": f"{state['ws_base']}/ws/terminal/{session_id}",
            "connection_ticket": record["ticket"],
            "ticket_expires_at": "2026-08-22T12:00:30Z",
        }
    )


async def terminate_session(request: web.Request) -> web.Response:
    require_control_headers(request)
    record = request.app["state"]["sessions"][request.match_info["session_id"]]
    record["closed"] = True
    request.app["state"]["cleanup_count"] += 1
    return response(session_payload(record))


async def terminal(request: web.Request) -> web.WebSocketResponse:
    state = request.app["state"]
    record = state["sessions"][request.match_info["session_id"]]
    assert request.headers.get("x-d1v-shell-ticket") == record["ticket"]
    assert request.headers.get("Authorization") is None
    assert "ticket=" not in str(request.rel_url)
    websocket = web.WebSocketResponse(protocols=(SUBPROTOCOL,))
    await websocket.prepare(request)
    assert websocket.ws_protocol == SUBPROTOCOL

    first = await websocket.receive()
    assert first.type == WSMsgType.TEXT
    opened = json.loads(first.data)
    assert opened == {
        "type": "open",
        "version": 1,
        "cols": opened["cols"],
        "rows": opened["rows"],
        "term": "xterm-256color",
    }
    await websocket.send_json(
        {"type": "ready", "session_id": record["session_id"], "cwd": record["cwd"]},
        dumps=lambda value: json.dumps(value, separators=(",", ":")),
    )

    if record["body"]["mode"] == "exec":
        argv = record["body"].get("argv") or []
        exit_code = 23 if "exit-23" in argv else 0
        await websocket.send_bytes(b"\x01exec-out-\x00-\xe2\x98\x83")
        await websocket.send_bytes(b"\x02exec-err")
        await websocket.send_json(
            {"type": "exit", "code": exit_code, "signal": None},
            dumps=lambda value: json.dumps(value, separators=(",", ":")),
        )
        return websocket

    state["interactive_ready"].set()
    async for message in websocket:
        if message.type == WSMsgType.TEXT:
            control = json.loads(message.data)
            record["controls"].append(control)
            if control.get("type") == "resize":
                state["resize_seen"].set()
            if control.get("type") == "detach":
                break
        elif message.type == WSMsgType.BINARY:
            assert message.data[:1] == b"\x00"
            payload = bytes(message.data[1:])
            record["inputs"].append(payload)
            if b"\x03" in payload:
                state["interrupt_seen"].set()
            if b"exit" in payload:
                await websocket.send_json(
                    {"type": "exit", "code": 0, "signal": None},
                    dumps=lambda value: json.dumps(value, separators=(",", ":")),
                )
                state["exit_sent"].set()
                await asyncio.sleep(0.25)
                await websocket.close()
                return websocket
    return websocket


async def start_server() -> tuple[web.Application, web.AppRunner, str]:
    app = web.Application()
    app["state"] = {
        "sessions": {},
        "cleanup_count": 0,
        "interactive_ready": asyncio.Event(),
        "resize_seen": asyncio.Event(),
        "interrupt_seen": asyncio.Event(),
        "exit_sent": asyncio.Event(),
        "ws_base": "",
    }
    app.router.add_get("/api/user/info", current_user)
    app.router.add_post("/api/workspace/shell-sessions", create_session)
    app.router.add_post("/api/projects/{project_id}/shell-sessions", create_session)
    app.router.add_delete("/api/shell-sessions/{session_id}", terminate_session)
    app.router.add_get("/ws/terminal/{session_id}", terminal)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", 0)
    await site.start()
    sockets = site._server.sockets
    port = int(sockets[0].getsockname()[1])
    base_url = f"http://127.0.0.1:{port}"
    app["state"]["ws_base"] = f"ws://127.0.0.1:{port}"
    return app, runner, base_url


async def run_command(
    binary: Path,
    base_url: str,
    home: Path,
    *arguments: str,
    token: str = VALID_TOKEN,
) -> tuple[int, bytes, bytes]:
    env = os.environ.copy()
    env["HOME"] = str(home)
    env["D1V_AUTH_TOKEN"] = token
    env.pop("D1V_API_KEY", None)
    env["D1V_LOG_FILE"] = str(home / "d1v.log")
    process = await asyncio.create_subprocess_exec(
        str(binary),
        "--base-url",
        base_url,
        *arguments,
        env=env,
        stdin=asyncio.subprocess.DEVNULL,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await asyncio.wait_for(process.communicate(), timeout=20)
    return int(process.returncode or 0), stdout, stderr


async def verify_exec(
    binary: Path, base_url: str, home: Path, app: web.Application
) -> None:
    code, stdout, stderr = await run_command(
        binary,
        base_url,
        home,
        "--format",
        "json",
        "exec",
        "--project-id",
        "project-e2e",
        "--",
        "success",
    )
    assert code == 0, stderr.decode(errors="replace")
    payload = json.loads(stdout)
    assert payload["project_id"] == "project-e2e"
    assert payload["cwd"] == "/workspace-root/projects/project-e2e"
    assert payload["exit_code"] == 0
    assert payload["stdout"] == "exec-out-\u0000-\u2603"
    assert payload["stderr"] == "exec-err"
    assert TICKET_PREFIX.encode() not in stdout + stderr

    code, stdout, stderr = await run_command(
        binary,
        base_url,
        home,
        "--format",
        "json",
        "exec",
        "--workspace",
        "--organization-id",
        "42",
        "--",
        "exit-23",
    )
    assert code == 23, stderr.decode(errors="replace")
    payload = json.loads(stdout)
    assert payload["project_id"] is None
    assert payload["exit_code"] == 23
    assert TICKET_PREFIX.encode() not in stdout + stderr

    sessions = list(app["state"]["sessions"].values())
    assert sessions[0]["body"]["argv"] == ["success"]
    assert sessions[1]["organization_id"] == "42"
    assert sessions[1]["body"]["argv"] == ["exit-23"]
    assert all(record["closed"] for record in sessions[:2])


def set_window_size(fd: int, rows: int, cols: int) -> None:
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def read_pty(fd: int) -> bytes:
    chunks: list[bytes] = []
    os.set_blocking(fd, False)
    while True:
        try:
            chunk = os.read(fd, 65536)
        except BlockingIOError:
            break
        except OSError as exc:
            if exc.errno == errno.EIO:
                break
            raise
        if not chunk:
            break
        chunks.append(chunk)
    return b"".join(chunks)


async def verify_interactive_shell(
    binary: Path,
    base_url: str,
    home: Path,
    app: web.Application,
) -> None:
    master_fd, slave_fd = pty.openpty()
    set_window_size(slave_fd, 40, 120)
    before = termios.tcgetattr(slave_fd)
    env = os.environ.copy()
    env["HOME"] = str(home)
    env["D1V_AUTH_TOKEN"] = VALID_TOKEN
    env.pop("D1V_API_KEY", None)
    env["D1V_LOG_FILE"] = str(home / "d1v.log")
    process = await asyncio.create_subprocess_exec(
        str(binary),
        "--base-url",
        base_url,
        "shell",
        env=env,
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
    )
    try:
        await asyncio.wait_for(app["state"]["interactive_ready"].wait(), timeout=10)
        await asyncio.sleep(0.2)
        os.write(master_fd, b"\x03")
        await asyncio.wait_for(app["state"]["interrupt_seen"].wait(), timeout=5)
        set_window_size(slave_fd, 52, 132)
        await asyncio.wait_for(app["state"]["resize_seen"].wait(), timeout=5)
        os.write(master_fd, b"exit\n")
        try:
            await asyncio.wait_for(process.wait(), timeout=10)
        except TimeoutError as exc:
            interactive = list(app["state"]["sessions"].values())[-1]
            output = read_pty(master_fd).decode(errors="replace")
            raise RuntimeError(
                "interactive shell did not exit: "
                f"controls={interactive['controls']!r} "
                f"inputs={interactive['inputs']!r} "
                f"exit_sent={app['state']['exit_sent'].is_set()} "
                f"cleanup_count={app['state']['cleanup_count']} "
                f"returncode={process.returncode!r} output={output!r}"
            ) from exc
    finally:
        if process.returncode is None:
            process.kill()
            await process.wait()
    after = termios.tcgetattr(slave_fd)
    output = read_pty(master_fd)
    os.close(master_fd)
    os.close(slave_fd)
    assert process.returncode == 0, output.decode(errors="replace")
    assert before[3] & (termios.ECHO | termios.ICANON) == after[3] & (
        termios.ECHO | termios.ICANON
    )
    interactive = list(app["state"]["sessions"].values())[-1]
    assert any(control.get("type") == "resize" for control in interactive["controls"])
    assert any(b"\x03" in value for value in interactive["inputs"])
    assert interactive["closed"] is True
    assert TICKET_PREFIX.encode() not in output


async def verify_auth_and_non_tty(binary: Path, base_url: str, home: Path) -> None:
    code, _, _ = await run_command(
        binary, base_url, home, "exec", "--", "success", token=EXPIRED_TOKEN
    )
    assert code == 4

    code, _, stderr = await run_command(binary, base_url, home, "shell")
    assert code == 1
    assert b"interactive terminal" in stderr

    code, _, stderr = await run_command(
        binary, base_url, home, "--format", "json", "shell"
    )
    assert code == 1
    assert b"text output" in stderr


async def async_main(args: argparse.Namespace) -> int:
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)
    app, runner, base_url = await start_server()
    try:
        with tempfile.TemporaryDirectory(prefix="d1v-shell-e2e-") as temp_dir:
            home = Path(temp_dir)
            await verify_exec(binary, base_url, home, app)
            await verify_auth_and_non_tty(binary, base_url, home)
            await verify_interactive_shell(binary, base_url, home, app)
            assert app["state"]["cleanup_count"] == 3
    finally:
        await runner.cleanup()
    print("d1v shell/exec process E2E passed")
    return 0


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(
        description="Run d1v shell/exec process E2E checks."
    )
    value.add_argument("binary", help="Path to the compiled d1v binary")
    return value


if __name__ == "__main__":
    raise SystemExit(asyncio.run(async_main(parser().parse_args())))
