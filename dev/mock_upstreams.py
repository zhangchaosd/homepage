#!/usr/bin/env python3
"""Fake upstream services for developing the homepage without the real ones.

    python3 dev/mock_upstreams.py          # listens on 127.0.0.1:8901

Serves ipv6who, Vikunja, Forgejo, bookmarkd and Home Assistant shaped
responses under path prefixes; see dev/config/config.toml.
"""

import json
import random
import struct
import zlib
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = 8901
NOW = datetime.now().astimezone()


def iso(dt):
    return dt.isoformat(timespec="seconds")


def at(days, hour=18, minute=0):
    day = (NOW + timedelta(days=days)).replace(hour=hour, minute=minute, second=0, microsecond=0)
    return iso(day)


DEVICES = {
    "nas": {"address": "2408:8207:2c41:5a10::10", "interface": "eth0", "type": "static",
            "extra": [("2408:8207:2c41:5a10:be24:11ff:fe8a:3c21", "dynamic")],
            "system": {"hostname": "nas.lan", "os": "linux", "name": "Fedora Linux 44 (Cloud Edition)", "version": "44", "arch": "x86_64"}},
    "server": {"address": "2408:8207:2c41:5a10:5054:ff:fe12:a7b3", "interface": "enp3s0", "type": "dynamic", "extra": [],
               "system": {"hostname": "Mac_mini_M5Pro.lan", "os": "macos", "name": "macOS 27.0.1", "version": "27.0.1", "arch": "aarch64"}},
    "desktop": {"address": "2408:8207:2c41:5a10:9d3e:41c2:7f0a:12e8", "interface": "Ethernet", "type": "dynamic", "extra": [],
                "system": {"hostname": "Windows11onPVE", "os": "windows", "name": "Windows 11 Pro 25H2", "version": "10.0.26200", "arch": "x86_64"}},
    # No global address -> 503, which still reports the host.
    "pi": {"address": None, "system": {"hostname": "raspberrypi", "os": "linux", "name": "Debian GNU/Linux 12 (bookworm)", "version": "12", "arch": "aarch64"}},
}

TASKS = [
    {"id": 101, "title": "续费域名 example.com", "due_date": at(-1, 23, 59), "priority": 4, "project_id": 1, "labels": [{"title": "运维", "hex_color": "e8590c"}]},
    {"id": 102, "title": "给 bookmarkd 加只读 API token", "due_date": at(0, 0, 0), "priority": 3, "project_id": 2, "labels": [{"title": "开发", "hex_color": "2f7f68"}]},
    {"id": 103, "title": "备份 Forgejo 数据库到 NAS", "due_date": at(0, 21, 30), "priority": 2, "project_id": 1, "labels": []},
    {"id": 104, "title": "整理 Obsidian 周报", "due_date": at(1, 10, 0), "priority": 1, "project_id": 3, "labels": []},
    {"id": 105, "title": "检查 Home Assistant 自动化日志", "due_date": at(3, 0, 0), "priority": 1, "project_id": 1, "labels": [{"title": "家", "hex_color": "3a74b8"}]},
    {"id": 106, "title": "买猫粮", "due_date": at(5, 0, 0), "priority": 0, "project_id": 4, "labels": []},
]
PROJECTS = [{"id": 1, "title": "基础设施"}, {"id": 2, "title": "Homepage"}, {"id": 3, "title": "写作"}, {"id": 4, "title": "生活"}]

COMMITS = [
    ("homepage", "main", "Render LAN IPv6 card with copy-to-clipboard", 0.3),
    ("homepage", "main", "Cache widget data in background refreshers", 2.1),
    ("bookmarkd", "read-token", "Add read-only bearer tokens for integrations", 5.5),
    ("ipv6who", "main", "Skip addresses still in DAD", 26),
    ("obsidian-web-gateway", "main", "Keep scroll position when switching modes", 49),
    ("dotfiles", "main", "zsh: faster prompt in large repos", 75),
    ("homepage", "main", "Initial commit", 98),
]

BOOKMARKS = [
    ("Rust 标准库", "https://doc.rust-lang.org/std/", ["rust"]),
    ("Svelte 5 文档", "https://svelte.dev/docs/svelte/overview", ["web"]),
    ("Hacker News", "https://news.ycombinator.com/", []),
    ("Cloudflare 控制台", "https://dash.cloudflare.com/", ["运维"]),
    ("Caddy 文档", "https://caddyserver.com/docs/", ["运维"]),
    ("MDN Web Docs", "https://developer.mozilla.org/zh-CN/", ["web"]),
    ("V2EX", "https://www.v2ex.com/", []),
    ("Tailscale", "https://login.tailscale.com/admin", ["网络"]),
    ("crates.io", "https://crates.io/", ["rust"]),
    ("少数派", "https://sspai.com/", []),
]

HA = {
    "sensor.living_room_temperature": ("23.6", {"friendly_name": "客厅温度", "unit_of_measurement": "°C", "device_class": "temperature"}),
    "sensor.living_room_humidity": ("48", {"friendly_name": "客厅湿度", "unit_of_measurement": "%", "device_class": "humidity"}),
    "sensor.home_power": ("386.4", {"friendly_name": "全屋功率", "unit_of_measurement": "W", "device_class": "power"}),
    "light.study": ("on", {"friendly_name": "书房灯"}),
    "binary_sensor.front_door": ("off", {"friendly_name": "入户门", "device_class": "door"}),
    "person.chao": ("home", {"friendly_name": "Chao"}),
}


INBOX = {"instance_id": "dfb5b8e8-9171-479e-aa8e-db2b23c13e02", "data_generation": "776aad66-abfe-4775-8405-4926568cfe47", "items": [
    {"key": "seed-1", "item_id": "5d0c3a0e-1111-4c1e-9a51-000000000001", "body": "电梯里想到的：主页加一个收件箱卡片，点开能看全文、置顶、改名、扔进回收站。\n\n顺便把待办也做成能直接添加。", "summary": {"title": "", "summary": "电梯里想到的：主页加一个收件箱卡片", "kind": "text", "created_at": iso(NOW - timedelta(minutes=35)),
     "pinned": True, "expires_at": None, "source": {"kind": "ios_share", "label": "iPhone"}, "attachment_count": 0}},
    {"key": "seed-2", "item_id": "5d0c3a0e-2222-4c1e-9a51-000000000002", "body": "反向代理配置参考 https://caddyserver.com/docs/caddyfile/directives/reverse_proxy", "links": [{"url": "https://caddyserver.com/docs/caddyfile/directives/reverse_proxy", "title": "reverse_proxy (Caddyfile directive)"}], "summary": {"title": "Caddy reverse_proxy 文档", "summary": "https://caddyserver.com/docs/caddyfile/directives/reverse_proxy", "kind": "link",
     "created_at": iso(NOW - timedelta(hours=5)), "pinned": False, "expires_at": iso(NOW + timedelta(hours=9)), "source": {"kind": "web", "label": None}, "attachment_count": 0}},
    {"key": "seed-3", "item_id": "5d0c3a0e-3333-4c1e-9a51-000000000003", "body": "", "files": [("发票-十月.jpg", (196, 104, 63)), ("收据.jpg", (58, 116, 184))], "summary": {"title": "发票扫描", "summary": "2 个附件", "kind": "image", "created_at": iso(NOW - timedelta(days=2)),
     "pinned": False, "expires_at": iso(NOW + timedelta(days=5)), "source": {"kind": "ios_app", "label": "iPad"}, "attachment_count": 2}},
]}
INBOX_TOKEN = "Bearer dev-inbox-token"
LABELS = [{"id": 1, "title": "运维", "hex_color": "e8590c"}, {"id": 2, "title": "开发", "hex_color": "2f7f68"}, {"id": 3, "title": "家", "hex_color": "3a74b8"}]


def png(rgb, w=160, h=120):
    """A small gradient PNG standing in for an inboxd preview."""
    rows = b"".join(b"\x00" + bytes(c for x in range(w) for c in (min(255, rgb[0] + x // 3), min(255, rgb[1] + y // 3), rgb[2])) for y in range(h))
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


def inbox_item(item_id):
    return next((i for i in INBOX["items"] if i["item_id"] == item_id), None)


def inbox_detail(item):
    files = item.get("files") or []
    return dict(item["summary"], id=item["item_id"], version=item.setdefault("version", 1), body_text=item.get("body") or item["summary"]["summary"],
                links=item.get("links") or [], state=item.get("state", "active"), retention_mode="expires",
                attachments=[{"id": "%sa%03d" % (item["item_id"][:-4], n), "item_id": item["item_id"], "ordinal": n, "original_filename": name,
                              "declared_media_type": "image/jpeg", "detected_media_type": "image/jpeg", "source_uti": None,
                              "byte_length": 182000 + n * 41000, "sha256": "0" * 64, "preview_status": "ready", "preview_byte_length": 9000}
                             for n, (name, _) in enumerate(files)])


def heatmap():
    random.seed(7)
    out = []
    today = NOW.replace(hour=12, minute=0, second=0, microsecond=0)
    for d in range(200):
        if random.random() < 0.45:
            continue
        day = today - timedelta(days=d)
        out.append({"timestamp": int(day.timestamp()), "contributions": random.choice([1, 1, 2, 3, 4, 6, 9])})
    return out


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass

    def send(self, status, body=None):
        data = json.dumps(body, ensure_ascii=False).encode() if body is not None else b""
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = self.path.split("?")[0]
        parts = path.strip("/").split("/")
        if path == "/up":
            return self.send(200, {"ok": True})
        if parts[0] == "ipv6" and len(parts) >= 3:
            device = DEVICES.get(parts[1])
            if device is None:
                return self.send(404, {"error": "unknown device"})
            if device["address"] is None:
                return self.send(503, {"error": "no global IPv6 address found", "addresses": [], "system": device["system"]})
            addresses = [{"address": device["address"], "prefix_len": 64, "interface": device["interface"], "type": device["type"],
                          "scope": "global", "deprecated": False}]
            addresses += [{"address": a, "prefix_len": 64, "interface": device["interface"], "type": t, "scope": "global",
                           "deprecated": False} for a, t in device["extra"]]
            return self.send(200, {"address": device["address"], "interface": device["interface"], "type": device["type"],
                                   "addresses": addresses, "system": device["system"]})
        if path == "/inbox/health/ready":
            return self.send(200, {"status": "ok"})
        if path == "/inbox/api/v1/capabilities":
            return self.send(200, {"api_version": 1, "instance_id": INBOX["instance_id"], "data_generation": INBOX["data_generation"]})
        if path == "/inbox/api/v1/me":
            if self.headers.get("authorization") != INBOX_TOKEN:
                return self.send(401, {"error": {"code": "authentication_required"}})
            return self.send(200, {"authentication": "script", "grant_id": "g1", "expires_at": iso(NOW + timedelta(days=21)),
                                   "scopes": ["ingest:write", "receipts:read_own", "items:read", "items:write", "items:trash"], "max_ttl_seconds": 3153600000})
        if path == "/inbox/api/v1/items":
            if self.headers.get("authorization") != INBOX_TOKEN:
                return self.send(401, {"error": {"code": "authentication_required"}})
            items = sorted((dict(i["summary"], id=i["item_id"], version=i.get("version", 1), preview_attachment_id=None) for i in INBOX["items"]
                            if i.get("state", "active") == "active"),
                           key=lambda i: i["created_at"], reverse=True)
            return self.send(200, {"items": items[:6], "next_cursor": None})
        if parts[:4] == ["inbox", "api", "v1", "items"] and len(parts) == 5:
            if self.headers.get("authorization") != INBOX_TOKEN:
                return self.send(401, {"error": {"code": "authentication_required"}})
            item = inbox_item(parts[4])
            if not item or item.get("state", "active") != "active":
                return self.send(404, {"error": {"code": "not_found"}})
            return self.send(200, inbox_detail(item))
        if parts[:4] == ["inbox", "api", "v1", "attachments"] and len(parts) == 6:
            owner = next((i for i in INBOX["items"] if i.get("files") and parts[4][:-4] == i["item_id"][:-4]), None)
            if not owner:
                return self.send(404, {"error": {"code": "not_found"}})
            name, rgb = owner["files"][int(parts[4][-3:])]
            data = png(rgb)
            self.send_response(200)
            self.send_header("content-type", "image/jpeg" if parts[5] == "preview" else "application/octet-stream")
            if parts[5] == "content":
                self.send_header("content-disposition", "attachment; filename*=UTF-8''" + __import__("urllib.parse").parse.quote(name))
            self.send_header("content-length", str(len(data)))
            self.end_headers()
            return self.wfile.write(data)
        if path == "/vikunja/api/v1/labels":
            return self.send(200, LABELS)
        if path == "/vikunja/api/v1/user":
            return self.send(200, {"id": 1, "username": "chao", "settings": {"default_project_id": 4}})
        if path == "/vikunja/api/v1/tasks/all":
            # Vikunja 2.x removed this route; it now parses "all" as a task id.
            return self.send(400, {"code": 2004, "message": "Invalid model provided: Bad Request"})
        if path == "/vikunja/api/v1/tasks":
            return self.send(200, [dict(t, done=False, identifier=f"#{t['id']}") for t in TASKS])
        if path == "/vikunja/api/v1/projects":
            return self.send(200, PROJECTS)
        if path.startswith("/vikunja/api/v1/tasks/"):
            task_id = int(parts[-1])
            task = next((t for t in TASKS if t["id"] == task_id), None)
            return self.send(200, dict(task, done=False)) if task else self.send(404, {})
        if path == "/forgejo/api/v1/users/chao/activities/feeds":
            feed = []
            for repo, branch, message, hours in COMMITS:
                sha = "%040x" % random.getrandbits(160)
                ts = iso(NOW - timedelta(hours=hours))
                content = json.dumps({"Commits": [{"Sha1": sha, "Message": message + "\n", "AuthorName": "chao", "Timestamp": ts}], "Len": 1})
                feed.append({"op_type": "commit_repo", "ref_name": f"refs/heads/{branch}", "repo": {"full_name": f"chao/{repo}"},
                             "content": content, "created": ts})
            return self.send(200, feed)
        if path == "/forgejo/api/v1/users/chao/heatmap":
            return self.send(200, heatmap())
        if path == "/bookmarkd/api/v1/bookmarks":
            if self.headers.get("authorization") != "Bearer dev-bookmarkd-token":
                return self.send(401, {"error": {"code": "UNAUTHENTICATED"}})
            items = [{"id": f"b{i}", "url_raw": url, "title": title, "tags": tags, "pinned": True, "pinned_position": i, "icon": None}
                     for i, (title, url, tags) in enumerate(BOOKMARKS)]
            return self.send(200, {"items": items, "total": len(items), "revision": 1})
        if parts[:3] == ["ha", "api", "states"]:
            entity = HA.get(parts[3])
            if not entity:
                return self.send(404, {"message": "Entity not found."})
            state, attrs = entity
            return self.send(200, {"entity_id": parts[3], "state": state, "attributes": attrs, "last_changed": iso(NOW - timedelta(minutes=12))})
        return self.send(404, {"error": "not found"})

    def do_POST(self):
        path = self.path.split("?")[0]
        length = int(self.headers.get("content-length") or 0)
        body = json.loads(self.rfile.read(length) or b"{}")
        if path == "/inbox/api/v1/ingest":
            # Mirrors inboxd's checks that matter to the homepage.
            if self.headers.get("authorization") != INBOX_TOKEN:
                return self.send(401, {"error": {"code": "authentication_required"}})
            for name in ("instance_id", "data_generation"):
                header = self.headers.get("x-inbox-" + name.replace("_", "-"))
                if header is not None and header != INBOX[name]:
                    return self.send(409, {"error": {"code": "destination_changed"}})
            key = self.headers.get("idempotency-key")
            if not key or body.get("client_submission_id") != key or body.get("schema_version") != 1:
                return self.send(400, {"error": {"code": "invalid_request"}})
            existing = next((i for i in INBOX["items"] if i["key"] == key), None)
            if existing is None:
                links = body.get("links") or []
                existing = {"key": key, "item_id": "%08x-0000-4000-8000-%012x" % (len(INBOX["items"]), len(INBOX["items"])), "manifest": body,
                            "summary": {"title": body.get("title") or "", "summary": (body.get("body_text") or "")[:120], "kind": "link" if links else "text",
                                        "created_at": iso(datetime.now().astimezone()), "pinned": False, "expires_at": iso(NOW + timedelta(days=7)),
                                        "source": body.get("source"), "attachment_count": 0}}
                INBOX["items"].append(existing)
                print("inbox received:", json.dumps({k: body.get(k) for k in ("title", "body_text", "links", "source")}, ensure_ascii=False), flush=True)
            return self.send(201, {"receipt": {"client_submission_id": key, "item_id": existing["item_id"], "status": "committed",
                                               "committed_at": iso(NOW), "purged_at": None}})
        parts = path.strip("/").split("/")
        if parts[:4] == ["inbox", "api", "v1", "items"] and len(parts) == 6 and parts[5] in ("trash", "restore"):
            item = inbox_item(parts[4])
            if not item:
                return self.send(404, {"error": {"code": "not_found"}})
            if self.headers.get("if-match") != '"%d"' % item.get("version", 1):
                return self.send(412, {"error": {"code": "revision_conflict"}})
            item["state"] = "trash" if parts[5] == "trash" else "active"
            item["version"] = item.get("version", 1) + 1
            return self.send(200, inbox_detail(item))
        if parts[-2:] == ["labels", "bulk"]:
            task = next((t for t in TASKS if t["id"] == int(parts[-3])), None)
            if not task:
                return self.send(404, {})
            task["labels"] = [dict((l for l in LABELS if l["id"] == x["id"]).__next__()) for x in body.get("labels", [])]
            return self.send(201, body)
        if path.startswith("/vikunja/api/v1/tasks/"):
            task_id = int(path.rsplit("/", 1)[-1])
            if body.get("done") and body.get("title"):
                TASKS[:] = [t for t in TASKS if t["id"] != task_id]
                return self.send(200, body)
            return self.send(400, {"message": "full task required"})
        return self.send(404, {})

    def do_PUT(self):
        path = self.path.split("?")[0]
        parts = path.strip("/").split("/")
        body = json.loads(self.rfile.read(int(self.headers.get("content-length") or 0)) or b"{}")
        if path == "/vikunja/api/v1/labels":
            label = {"id": max(l["id"] for l in LABELS) + 1, "title": body["title"], "hex_color": body.get("hex_color", "")}
            LABELS.append(label)
            return self.send(201, label)
        if parts[:4] == ["vikunja", "api", "v1", "projects"] and parts[-1] == "tasks":
            task = {"id": max(t["id"] for t in TASKS) + 1, "title": body["title"], "due_date": body.get("due_date") or "0001-01-01T00:00:00Z",
                    "priority": body.get("priority", 0), "project_id": int(parts[4]), "labels": []}
            TASKS.append(task)
            print("vikunja task created:", json.dumps(body, ensure_ascii=False), flush=True)
            return self.send(201, task)
        return self.send(404, {})

    def do_PATCH(self):
        path = self.path.split("?")[0]
        parts = path.strip("/").split("/")
        body = json.loads(self.rfile.read(int(self.headers.get("content-length") or 0)) or b"{}")
        if parts[:4] == ["inbox", "api", "v1", "items"] and len(parts) == 5:
            item = inbox_item(parts[4])
            if not item:
                return self.send(404, {"error": {"code": "not_found"}})
            if self.headers.get("if-match") != '"%d"' % item.get("version", 1):
                return self.send(412, {"error": {"code": "revision_conflict"}})
            if "retention" in body:
                return self.send(403, {"error": {"code": "insufficient_scope"}})
            for key in ("title", "pinned"):
                if key in body:
                    item["summary"][key] = body[key]
            item["version"] = item.get("version", 1) + 1
            return self.send(200, inbox_detail(item))
        return self.send(404, {})


if __name__ == "__main__":
    print(f"mock upstreams on http://127.0.0.1:{PORT}")
    # The homepage probes everything at once; the default backlog of 5 drops connections.
    ThreadingHTTPServer.request_queue_size = 128
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
