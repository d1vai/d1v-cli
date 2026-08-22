#!/usr/bin/env python3
from __future__ import annotations

import asyncio
import json
import os
import signal
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from aiohttp import ClientSession, WSMsgType, web


class _OpcodeHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        if self.path == "/health":
            self._write_json({"status": "ok", "service": "opcode-api"})
            return
        if self.path == "/api/d1v/runtime/capabilities":
            self._write_json(
                {
                    "supports_terminal": True,
                    "terminal_base_url": "http://127.0.0.1:9191",
                }
            )
            return
        if self.path == "/api/d1v/runtime/projects":
            self._write_json(
                [
                    {
                        "path": "/tmp/e2e-home/projects/demo",
                        "name": "demo",
                        "project_id": "proj_e2e_demo",
                        "framework": "nextjs",
                        "package_manager": "pnpm",
                        "has_workspace_binding": True,
                        "is_bound_to_cloud_project": True,
                    }
                ]
            )
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, format: str, *args) -> None:
        return

    def _write_json(self, payload: object) -> None:
        body = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def start_opcode_server() -> tuple[ThreadingHTTPServer, threading.Thread]:
    server = ThreadingHTTPServer(("127.0.0.1", 9191), _OpcodeHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, thread


async def build_backend_app(done: asyncio.Event) -> web.Application:
    app = web.Application()
    app["state"] = {"events": [], "done": done}

    async def agent_connect(request: web.Request) -> web.WebSocketResponse:
        assert request.query.get("token") == "e2e-token"
        assert request.query.get("device_id") == "e2e-device-1"

        ws = web.WebSocketResponse()
        await ws.prepare(request)

        await ws.send_json(
            {
                "type": "request",
                "request_id": "req-projects",
                "method": "GET",
                "path": "/api/d1v/runtime/projects",
            }
        )

        async for message in ws:
            if message.type != WSMsgType.TEXT:
                continue
            payload = json.loads(message.data)
            request.app["state"]["events"].append(payload)
            if payload.get("type") == "response":
                assert payload["request_id"] == "req-projects"
                assert payload["status_code"] == 200
                body = payload["body"]
                assert isinstance(body, list) and body[0]["project_id"] == "proj_e2e_demo"
                done.set()
                break

        return ws

    async def runtime_bootstrap(request: web.Request) -> web.Response:
        assert request.headers.get("Authorization") == "Bearer e2e-token"
        return web.json_response(
            {
                "code": 0,
                "msg": "success",
                "data": {
                    "enabled": True,
                    "public_key": "e2e-public-key",
                    "issuer": "d1v-control-plane",
                    "audience": "d1v-runtime-terminal",
                },
                "total": None,
            }
        )

    async def register_runtime_node(request: web.Request) -> web.Response:
        assert request.headers.get("Authorization") == "Bearer e2e-token"
        payload = await request.json()
        assert payload["node_id"] == "customer-e2e-device-1"
        assert payload["capabilities"] == {
            "runtime": "opcode-api",
            "transport": "relay",
            "device_id": "e2e-device-1",
            "supports_terminal": True,
            "terminal_base_url": "http://127.0.0.1:9191",
        }
        request.app["state"]["registration_done"].set()
        return web.json_response(
            {"code": 0, "msg": "success", "data": {"accepted": True}}
        )

    app.router.add_get("/api/agent/connect", agent_connect)
    app.router.add_get("/api/devices/runtime/bootstrap", runtime_bootstrap)
    app.router.add_post("/api/devices/runtime-node/register", register_runtime_node)
    return app


async def start_site(app: web.Application, port: int) -> web.AppRunner:
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", port)
    await site.start()
    return runner


async def wait_for_http(url: str, timeout: float = 15.0) -> None:
    deadline = asyncio.get_running_loop().time() + timeout
    async with ClientSession() as session:
        while True:
            try:
                async with session.get(url) as response:
                    if response.status < 500:
                        return
            except Exception:
                pass
            if asyncio.get_running_loop().time() >= deadline:
                raise RuntimeError(f"timed out waiting for {url}")
            await asyncio.sleep(0.25)


async def main() -> int:
    if len(sys.argv) != 2:
        print("usage: test-agent-relay-e2e.py <d1v-binary>", file=sys.stderr)
        return 2

    d1v_bin = Path(sys.argv[1]).resolve()
    if not d1v_bin.exists():
        raise FileNotFoundError(d1v_bin)

    done = asyncio.Event()
    opcode_server, opcode_thread = start_opcode_server()
    backend_app = await build_backend_app(done)
    backend_app["state"]["registration_done"] = asyncio.Event()
    backend_runner = await start_site(backend_app, 18080)

    temp_home = Path(tempfile.mkdtemp(prefix="d1v-agent-e2e-"))
    agent_home = temp_home / "agent-home"
    env = os.environ.copy()
    env["HOME"] = str(temp_home)
    env["D1V_AUTH_TOKEN"] = "e2e-token"

    await wait_for_http("http://127.0.0.1:9191/health")

    init = await asyncio.create_subprocess_exec(
        str(d1v_bin),
        "--base-url",
        "http://127.0.0.1:18080",
        "agent",
        "init-home",
        "--path",
        str(agent_home),
        "--device-id",
        "e2e-device-1",
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    init_output, _ = await init.communicate()
    if init.returncode != 0:
        raise RuntimeError(init_output.decode())

    proc = await asyncio.create_subprocess_exec(
        str(d1v_bin),
        "--base-url",
        "http://127.0.0.1:18080",
        "agent",
        "start",
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )

    timed_out = False
    try:
        try:
            await asyncio.wait_for(
                asyncio.gather(
                    done.wait(), backend_app["state"]["registration_done"].wait()
                ),
                timeout=20.0,
            )
        except asyncio.TimeoutError:
            timed_out = True
    finally:
        if proc.returncode is None:
            proc.send_signal(signal.SIGTERM)
            try:
                await asyncio.wait_for(proc.wait(), timeout=5.0)
            except asyncio.TimeoutError:
                proc.kill()
                await proc.wait()

        await backend_runner.cleanup()
        opcode_server.shutdown()
        opcode_server.server_close()
        opcode_thread.join(timeout=2.0)

    output = ""
    if proc.stdout:
        output = (await proc.stdout.read()).decode()
    if (
        timed_out
        or not done.is_set()
        or not backend_app["state"]["registration_done"].is_set()
    ):
        raise RuntimeError(
            f"agent relay e2e timed out: events={backend_app['state']['events']} output={output}"
        )
    if proc.returncode not in (0, -15):
        raise RuntimeError(f"d1v agent start failed: {output}")

    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
