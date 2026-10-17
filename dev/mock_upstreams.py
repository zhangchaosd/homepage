#!/usr/bin/env python3
"""Fake upstream services for developing the homepage without the real ones.

    python3 dev/mock_upstreams.py          # listens on 127.0.0.1:8901

Serves ipv6who, Vikunja, Forgejo, bookmarkd and Home Assistant shaped
responses under path prefixes; see dev/config/config.toml.
"""

import json
import random
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
            "extra": [("2408:8207:2c41:5a10:be24:11ff:fe8a:3c21", "dynamic")]},
    "server": {"address": "2408:8207:2c41:5a10:5054:ff:fe12:a7b3", "interface": "enp3s0", "type": "dynamic", "extra": []},
    "desktop": {"address": "2408:8207:2c41:5a10:9d3e:41c2:7f0a:12e8", "interface": "Ethernet", "type": "dynamic", "extra": []},
    "pi": None,  # no global address -> 503
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
                return self.send(503, {"error": "no global IPv6 address found", "addresses": []})
            addresses = [{"address": device["address"], "prefix_len": 64, "interface": device["interface"], "type": device["type"],
                          "scope": "global", "deprecated": False}]
            addresses += [{"address": a, "prefix_len": 64, "interface": device["interface"], "type": t, "scope": "global",
                           "deprecated": False} for a, t in device["extra"]]
            return self.send(200, {"address": device["address"], "interface": device["interface"], "type": device["type"],
                                   "addresses": addresses})
        if path in ("/vikunja/api/v1/tasks/all", "/vikunja/api/v1/tasks"):
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
        if path.startswith("/vikunja/api/v1/tasks/"):
            task_id = int(path.rsplit("/", 1)[-1])
            if body.get("done") and body.get("title"):
                TASKS[:] = [t for t in TASKS if t["id"] != task_id]
                return self.send(200, body)
            return self.send(400, {"message": "full task required"})
        return self.send(404, {})


if __name__ == "__main__":
    print(f"mock upstreams on http://127.0.0.1:{PORT}")
    # The homepage probes everything at once; the default backlog of 5 drops connections.
    ThreadingHTTPServer.request_queue_size = 128
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
