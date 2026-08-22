#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import base64
import json
import os
import re
import signal
import socket
import subprocess
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any

import jwt
from aiohttp import WSMsgType, web


SUBPROTOCOL = "d1v-terminal.v1"
PRIVATE_KEY = """-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIGk7UIa30856KWMiFuojvX0gaHPJk9Fyo5xL5z0ruHKI
-----END PRIVATE KEY-----
"""
PUBLIC_KEY = """-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA6nql11TopNIvXEcadKmm9/vxanBhdc85k1c5VSjrDzw=
-----END PUBLIC KEY-----
"""


def api_response(data: Any) -> web.Response:
    return web.json_response({"code": 0, "msg": "success", "data": data})


def terminal_ticket(session_id: str, *, jti: str) -> str:
    now = int(time.time())
    return jwt.encode(
        {
            "iss": "d1v-control-plane",
            "aud": "d1v-runtime-terminal",
            "iat": now,
            "exp": now + 60,
            "jti": jti,
            "session_id": session_id,
            "actor_user_id": 7,
            "workspace_user_id": 7,
            "opcode_username": "local-user",
            "workspace_scope": "user:7",
            "organization_id": None,
            "project_id": "project-1",
            "node_id": "customer-e2e-terminal-device",
            "container_name": "d1v-runtime-local-user",
            "cwd": "/workspace-root/projects/project-1",
            "permissions": ["shell:open", "shell:write"],
            "mode": "pty",
            "argv": [],
        },
        PRIVATE_KEY,
        algorithm="EdDSA",
        headers={"kid": "d1v-shell-e2e", "typ": "JWT"},
    )


class RelayTerminal:
    def __init__(self, websocket: web.WebSocketResponse, tunnel_id: str) -> None:
        self.websocket = websocket
        self.tunnel_id = tunnel_id
        self.stdout = bytearray()

    async def send_text(self, payload: dict[str, Any]) -> None:
        await self.websocket.send_json(
            {
                "type": "ws_send",
                "tunnel_id": self.tunnel_id,
                "text": json.dumps(payload, separators=(",", ":")),
            }
        )

    async def send_input(self, payload: bytes) -> None:
        frame = b"\x00" + payload
        await self.websocket.send_json(
            {
                "type": "ws_send",
                "tunnel_id": self.tunnel_id,
                "bytes_base64": base64.b64encode(frame).decode("ascii"),
            }
        )

    async def receive(self, *, timeout: float = 10.0) -> dict[str, Any]:
        message = await asyncio.wait_for(self.websocket.receive(), timeout=timeout)
        if message.type != WSMsgType.TEXT:
            raise RuntimeError(f"unexpected relay message type {message.type}")
        payload = json.loads(message.data)
        if payload.get("type") != "ws_event":
            return payload
        if payload.get("tunnel_id") != self.tunnel_id:
            return payload
        if payload.get("event") == "bytes":
            frame = base64.b64decode(payload.get("bytes_base64") or "")
            if not frame or frame[0] not in (1, 2):
                raise RuntimeError("invalid terminal output frame")
            if frame[0] == 1:
                self.stdout.extend(frame[1:])
        return payload

    async def wait_event(self, event: str) -> dict[str, Any]:
        while True:
            payload = await self.receive()
            if (
                payload.get("type") == "ws_event"
                and payload.get("tunnel_id") == self.tunnel_id
                and payload.get("event") == event
            ):
                return payload

    async def wait_control(self, message_type: str) -> dict[str, Any]:
        while True:
            payload = await self.receive()
            if payload.get("event") != "text":
                continue
            control = json.loads(payload.get("text") or "{}")
            if control.get("type") == "error":
                raise RuntimeError(f"terminal error: {control.get('code')}")
            if control.get("type") == message_type:
                return control

    async def wait_output(self, marker: bytes, *, start: int = 0) -> bytes:
        while marker not in self.stdout[start:]:
            payload = await self.receive()
            if payload.get("event") == "text":
                control = json.loads(payload.get("text") or "{}")
                if control.get("type") in {"error", "exit"}:
                    raise RuntimeError("terminal ended before expected output")
        return bytes(self.stdout[start:])

    async def command(self, command: str) -> bytes:
        marker = f"__D1V_RELAY_{uuid.uuid4().hex}__"
        start = len(self.stdout)
        await self.send_input(
            f"{command}; __d1v_status=$?; printf '\\n{marker}:%s\\n' \"$__d1v_status\"\n".encode()
        )
        pattern = re.compile(rb"%s:(\d+)" % re.escape(marker.encode()))
        while True:
            output = bytes(self.stdout[start:])
            match = pattern.search(output)
            if match is not None:
                break
            payload = await self.receive()
            if payload.get("event") == "text":
                control = json.loads(payload.get("text") or "{}")
                if control.get("type") in {"error", "exit"}:
                    raise RuntimeError("terminal ended before command status")
        if match.group(1) != b"0":
            raise RuntimeError("terminal command returned a non-zero status")
        return output


async def relay_http(
    websocket: web.WebSocketResponse,
    *,
    method: str,
    path: str,
) -> dict[str, Any]:
    request_id = f"http-{uuid.uuid4().hex}"
    await websocket.send_json(
        {
            "type": "request",
            "request_id": request_id,
            "method": method,
            "path": path,
            "target_base_url": "http://127.0.0.1:9191",
            "query": {},
            "headers": {},
            "body_base64": "",
        }
    )
    while True:
        message = await asyncio.wait_for(websocket.receive(), timeout=10.0)
        if message.type != WSMsgType.TEXT:
            continue
        payload = json.loads(message.data)
        if payload.get("type") == "response" and payload.get("request_id") == request_id:
            return payload


async def exercise_terminal_relay(websocket: web.WebSocketResponse) -> None:
    session_id = "sh-local-relay-e2e"
    signed = terminal_ticket(session_id, jti="ticket-local-relay-e2e")
    tunnel = RelayTerminal(websocket, "terminal-primary")
    await websocket.send_json(
        {
            "type": "ws_open",
            "tunnel_id": tunnel.tunnel_id,
            "session_id": session_id,
            "target_base_url": "http://127.0.0.1:9191",
            "websocket_path": f"/ws/terminal/{session_id}",
            "headers": {"x-d1v-shell-ticket": signed},
            "subprotocols": [SUBPROTOCOL],
        }
    )
    await tunnel.wait_event("open")
    await tunnel.send_text(
        {
            "type": "open",
            "version": 1,
            "cols": 80,
            "rows": 24,
            "term": "xterm-256color",
        }
    )
    ready = await tunnel.wait_control("ready")
    if ready.get("cwd") != "/workspace-root/projects/project-1":
        raise RuntimeError("terminal opened in the wrong virtual cwd")

    output = await tunnel.command("pwd")
    if b"project-1" not in output:
        raise RuntimeError("terminal did not map the project cwd")
    await tunnel.send_text({"type": "resize", "cols": 132, "rows": 52})
    await asyncio.sleep(0.1)
    output = await tunnel.command("stty size")
    if b"52 132" not in output:
        raise RuntimeError("terminal resize did not reach the PTY")

    await tunnel.send_input(b"sleep 30\n")
    await asyncio.sleep(0.25)
    await tunnel.send_text({"type": "signal", "signal": "SIGINT"})
    output = await tunnel.command("printf 'after-interrupt\\n'")
    if b"after-interrupt" not in output:
        raise RuntimeError("terminal was unusable after Ctrl-C")

    completion_dir = f"/tmp/d1v-relay-{uuid.uuid4().hex}"
    completion_file = "nested-completion-target.txt"
    await tunnel.command(
        f"mkdir -p {completion_dir} && touch {completion_dir}/{completion_file}"
    )
    start = len(tunnel.stdout)
    await tunnel.send_input(f"cat {completion_dir}/nested-\t".encode())
    await tunnel.wait_output(completion_file.encode(), start=start)
    await tunnel.send_text({"type": "signal", "signal": "SIGINT"})

    status = await relay_http(
        websocket,
        method="GET",
        path=f"/control/runtime/terminal-sessions/{session_id}",
    )
    if status.get("status_code") != 200 or status.get("body", {}).get("status") != "ready":
        raise RuntimeError("terminal status relay failed")
    terminated = await relay_http(
        websocket,
        method="DELETE",
        path=f"/control/runtime/terminal-sessions/{session_id}",
    )
    if (
        terminated.get("status_code") != 200
        or terminated.get("body", {}).get("status") != "terminated"
    ):
        raise RuntimeError("terminal terminate relay failed")

    replay = RelayTerminal(websocket, "terminal-replay")
    await websocket.send_json(
        {
            "type": "ws_open",
            "tunnel_id": replay.tunnel_id,
            "session_id": session_id,
            "target_base_url": "http://127.0.0.1:9191",
            "websocket_path": f"/ws/terminal/{session_id}",
            "headers": {"x-d1v-shell-ticket": signed},
            "subprotocols": [SUBPROTOCOL],
        }
    )
    await replay.wait_event("open")
    await replay.send_text(
        {"type": "open", "version": 1, "cols": 80, "rows": 24}
    )
    while True:
        event = await replay.receive()
        if event.get("event") != "text":
            continue
        control = json.loads(event.get("text") or "{}")
        if control.get("type") == "error":
            if control.get("code") != "ticket_replayed":
                raise RuntimeError("ticket replay returned the wrong error")
            break


async def build_control_plane(done: asyncio.Event) -> web.Application:
    app = web.Application()
    app["state"] = {
        "done": done,
        "registered": asyncio.Event(),
        "exercise_started": False,
        "error": None,
    }

    async def bootstrap(request: web.Request) -> web.Response:
        if request.headers.get("Authorization") != "Bearer e2e-token":
            raise web.HTTPUnauthorized()
        return api_response(
            {
                "enabled": True,
                "public_key": PUBLIC_KEY,
                "issuer": "d1v-control-plane",
                "audience": "d1v-runtime-terminal",
            }
        )

    async def register_node(request: web.Request) -> web.Response:
        payload = await request.json()
        capabilities = payload.get("capabilities") or {}
        if payload.get("node_id") != "customer-e2e-terminal-device":
            raise web.HTTPBadRequest(text="wrong node id")
        if capabilities.get("supports_terminal") is not True:
            raise web.HTTPBadRequest(text="terminal capability missing")
        if capabilities.get("terminal_base_url") != "http://127.0.0.1:9191":
            raise web.HTTPBadRequest(text="terminal base URL mismatch")
        request.app["state"]["registered"].set()
        return api_response({"accepted": True})

    async def agent_connect(request: web.Request) -> web.WebSocketResponse:
        if request.query.get("token") != "e2e-token":
            raise web.HTTPUnauthorized()
        websocket = web.WebSocketResponse()
        await websocket.prepare(request)
        if request.app["state"]["exercise_started"]:
            await request.app["state"]["done"].wait()
            await websocket.close()
            return websocket
        request.app["state"]["exercise_started"] = True
        await request.app["state"]["registered"].wait()
        try:
            await exercise_terminal_relay(websocket)
        except Exception as error:
            request.app["state"]["error"] = error
        finally:
            request.app["state"]["done"].set()
            await websocket.close()
        return websocket

    async def noop(_request: web.Request) -> web.Response:
        return api_response({"ok": True})

    app.router.add_get("/api/devices/runtime/bootstrap", bootstrap)
    app.router.add_post("/api/devices/runtime-node/register", register_node)
    app.router.add_post("/api/devices/runtime-node/heartbeat", noop)
    app.router.add_get("/api/devices/runtime-node/exposes", noop)
    app.router.add_post("/api/devices/runtime-node/exposes", noop)
    app.router.add_get("/api/agent/connect", agent_connect)
    return app


async def start_site(app: web.Application) -> tuple[web.AppRunner, str]:
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", 0)
    await site.start()
    sockets = site._server.sockets
    port = int(sockets[0].getsockname()[1])
    return runner, f"http://127.0.0.1:{port}"


async def run_process(*args: str, env: dict[str, str]) -> tuple[int, str]:
    process = await asyncio.create_subprocess_exec(
        *args,
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    output, _ = await process.communicate()
    return int(process.returncode or 0), output.decode(errors="replace")


def child_processes(process_id: int) -> list[int]:
    try:
        output = subprocess.check_output(
            ["pgrep", "-P", str(process_id)], text=True, stderr=subprocess.DEVNULL
        )
    except (OSError, subprocess.CalledProcessError):
        return []
    return [int(value) for value in output.split() if value.isdigit()]


async def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("d1v_binary", type=Path)
    parser.add_argument("opcode_binary", type=Path)
    args = parser.parse_args()
    d1v_binary = args.d1v_binary.resolve()
    opcode_binary = args.opcode_binary.resolve()
    if not d1v_binary.exists() or not opcode_binary.exists():
        raise FileNotFoundError("d1v and opcode-api binaries are required")
    with socket.socket() as probe:
        if probe.connect_ex(("127.0.0.1", 9191)) == 0:
            raise RuntimeError("port 9191 is already in use")

    done = asyncio.Event()
    control_plane = await build_control_plane(done)
    runner, base_url = await start_site(control_plane)
    temp_home = Path(tempfile.mkdtemp(prefix="d1v-terminal-relay-e2e-"))
    workspace = temp_home / "runtime-home"
    env = os.environ.copy()
    env["HOME"] = str(temp_home)
    env["D1V_AUTH_TOKEN"] = "e2e-token"

    init_code, init_output = await run_process(
        str(d1v_binary),
        "--base-url",
        base_url,
        "agent",
        "init-home",
        "--path",
        str(workspace),
        "--device-id",
        "e2e-terminal-device",
        env=env,
    )
    if init_code != 0:
        raise RuntimeError(f"agent init failed: {init_output}")
    (workspace / "projects" / "project-1").mkdir(parents=True, exist_ok=True)

    agent = await asyncio.create_subprocess_exec(
        str(d1v_binary),
        "--base-url",
        base_url,
        "agent",
        "start",
        "--opcode-bin",
        str(opcode_binary),
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    opcode_children: list[int] = []
    try:
        await asyncio.wait_for(done.wait(), timeout=30.0)
        opcode_children = child_processes(agent.pid)
        if control_plane["state"]["error"] is not None:
            raise RuntimeError("terminal relay exercise failed") from control_plane["state"][
                "error"
            ]
    finally:
        if not opcode_children:
            opcode_children = child_processes(agent.pid)
        if agent.returncode is None:
            agent.send_signal(signal.SIGTERM)
            try:
                await asyncio.wait_for(agent.wait(), timeout=5.0)
            except asyncio.TimeoutError:
                agent.kill()
                await agent.wait()
        for process_id in opcode_children:
            try:
                os.kill(process_id, signal.SIGTERM)
            except ProcessLookupError:
                pass
        await runner.cleanup()
    if agent.returncode not in (0, -signal.SIGTERM):
        output = (await agent.stdout.read()).decode(errors="replace") if agent.stdout else ""
        raise RuntimeError(f"agent failed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
