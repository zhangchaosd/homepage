# Homepage

自托管服务的统一入口：一个单文件二进制 + 一个配置目录。

- **服务启动台**：一键打开 Forgejo、Vikunja、bookmarkd、Obsidian Web Gateway、Home Assistant…，每个服务带在线状态（服务端探测局域网地址）
- **局域网 IPv6**：通过各设备上的 [ipv6who](https://github.com/zhangchaosd/ipv6who) 显示当前全局 IPv6 地址，点击即可复制
- **今日待办**：Vikunja 中已逾期、今天和未来几天的任务，可直接勾选完成
- **最近提交**：Forgejo 动态里的提交 + 近半年贡献热力图
- **GitHub**：资料、最近推送的仓库，配置 token 后显示贡献日历
- **置顶书签**：bookmarkd 中置顶的书签（需要只读 token，见下文）
- **家**：Home Assistant 中指定实体的状态
- `/` 或 `⌘K` 搜索服务和书签，回车打开第一个结果
- **在线更新**：每天自动检查 GitHub Releases，右上角提示新版本，点"安装并重启"即可升级，无需重新部署

登录方式与 obsidian-web-gateway 相同：**用户名 + 密码**（Argon2）和/或 **Passkey**（复用 bookmarkd 的 `auth.db`）。整站都需要登录，未登录只能看到登录页。

## 工作方式

```
浏览器 ──HTTPS──> Caddy（网关机）──HTTP──> homepage（局域网）
                                            ├─ 后台定时拉取（各自间隔，失败保留旧数据）
                                            │   ipv6who × N / Vikunja / Forgejo / GitHub / bookmarkd / Home Assistant
                                            └─ 内嵌前端（Svelte），所有 token 只在服务端
```

浏览器只和 homepage 通信：token 不会下发，GitHub 头像也由服务端内联，CSP 保持 `default-src 'self'`。页面每 30 秒刷新一次缓存数据，标签页不可见时暂停。

## 安装

### Linux（systemd）

从 [GitHub Release](https://github.com/zhangchaosd/homepage/releases) 下载对应架构的单文件二进制（`uname -m` 为 `x86_64` 或 `aarch64`），然后安装为系统服务。服务以专用的 `homepage` 系统用户运行；程序目录 `/opt/homepage` 归它所有，这样页面上的在线更新才能替换二进制。Linux 版需要 glibc 2.39 或更新（Ubuntu 24.04+、Debian 13+）。

```sh
VERSION=0.1.0
PKG=homepage-$VERSION-linux-$(uname -m)
curl -fLO https://github.com/zhangchaosd/homepage/releases/download/v$VERSION/$PKG.tar.gz
tar xzf $PKG.tar.gz
sudo useradd --system --no-create-home --shell /usr/sbin/nologin homepage
sudo install -d -o homepage -g homepage -m 755 /opt/homepage
sudo install -o homepage -g homepage -m 755 $PKG/homepage /opt/homepage/homepage
sudo ln -sf /opt/homepage/homepage /usr/local/bin/homepage
sudo install -d -o homepage -g homepage -m 700 /etc/homepage
sudo install -o homepage -g homepage -m 600 $PKG/examples/config/config.toml /etc/homepage/config.toml
homepage hash-password | sudo install -o homepage -g homepage -m 600 /dev/stdin /etc/homepage/secrets.toml
sudoedit /etc/homepage/config.toml /etc/homepage/secrets.toml
sudo -u homepage homepage -c /etc/homepage check-config
sudo install -m 644 $PKG/examples/homepage.service /etc/systemd/system/homepage.service
sudo systemctl daemon-reload
sudo systemctl enable --now homepage
curl -s --retry 5 --retry-connrefused http://127.0.0.1:8780/healthz
```

`hash-password` 会提示输入两次登录密码，`secrets.toml` 里先只有密码哈希；在 `sudoedit` 中改好 `config.toml` 里的地址，并按 `$PKG/examples/config/secrets.toml.example` 往 `secrets.toml` 加上各服务的 token。`check-config` 通过后再启动服务。

配置：`/etc/homepage/config.toml` 和 `/etc/homepage/secrets.toml`。重启：`sudo systemctl restart homepage`。日志：`journalctl -u homepage`。以后升级在页面右上角一键完成（见[更新](#更新)），不需要重复这些步骤。服务运行在 systemd 沙箱里：只有 `/etc/homepage` 和 `/opt/homepage` 可写，没有任何 capability。

卸载：

```sh
sudo systemctl disable --now homepage
sudo rm -r /opt/homepage /etc/homepage /usr/local/bin/homepage /etc/systemd/system/homepage.service
sudo systemctl daemon-reload
sudo userdel homepage
```

### 从源码构建

需要 Rust 1.88+ 和 Node 20+（只在构建时需要）。产物是 `target/release/homepage`，用它替换上面步骤里解压出的 `$PKG/homepage` 即可。

```bash
scripts/build.sh
TARGET=x86_64-unknown-linux-gnu scripts/build.sh   # 交叉编译
```

## 配置

```
/etc/homepage/
├── config.toml     # 服务、设备、组件（参考 examples/config/config.toml）
├── secrets.toml    # 密码哈希和各种 token，权限 600
├── auth.db         # 从 bookmarkd 复制的 Passkey 数据库（可选）
├── icons/          # 自定义服务图标（可选）
└── state/          # 运行时写入（会话、更新状态），自动创建
```

### Passkey（复用 bookmarkd）

Passkey 绑定在 RP ID 上（例如 `example.com`），所以主页的 `public_url` 必须是这个域名或它的子域名，例如 `https://home.example.com`。

```sh
# 在 bookmarkd 所在机器上（sqlite3 .backup 保证拿到一致的副本）
sqlite3 /var/lib/bookmarkd/auth.db ".backup /tmp/auth.db"
scp /tmp/auth.db homepage-server:/tmp/auth.db
# 在 homepage 所在机器上
sudo install -o homepage -g homepage -m 600 /tmp/auth.db /etc/homepage/auth.db
sudoedit /etc/homepage/config.toml   # 取消 [auth] 里 passkey_db = "auth.db" 的注释
sudo systemctl restart homepage
```

注意：

- 两台机器各自持有一份副本，**在 bookmarkd 里吊销或新增 Passkey 后需要重新复制**。
- homepage 会写入 auth.db（签名计数器和本应用的会话行），所以文件和目录必须可写。
- 修改用户名、密码或 RP ID 会让所有已登录的浏览器退出。

### 反向代理

Caddy 在另一台网关机器上（见 `examples/Caddyfile`）：

```caddyfile
home.example.com {
	encode zstd gzip
	reverse_proxy 192.168.1.10:8780
}
```

在 `config.toml` 里：

```toml
[server]
listen = "0.0.0.0:8780"
public_url = "https://home.example.com"
trusted_proxies = ["192.168.1.2/32"]   # 网关机的局域网地址
```

用防火墙限制 8780 端口只允许网关机访问。`public_url` 为 https 时 Cookie 自动带 `Secure`。

## 更新

**发版**（在开发机上）：改 `Cargo.toml` 的 `version`，提交后打同名 tag 推送即可，GitHub Actions 会构建四个平台并发布 Release。tag 必须和 `Cargo.toml` 版本一致，否则工作流会失败（更新器会校验新二进制报告的版本号）。带 `-` 的 tag（如 `v0.2.0-beta.1`）发布为预发布版。

```bash
git commit -am "Release v0.2.0" && git tag v0.2.0 && git push origin main v0.2.0
```

**升级**（在主页上）：右上角齿轮按钮出现版本号时，点开"版本与更新"→"安装并重启"。服务端会：

1. 从 GitHub Releases 下载对应平台的压缩包，按 `SHA256SUMS.txt` 校验
2. 解出新二进制，运行 `--version` 确认版本正确
3. 原地替换，旧版本保留为 `homepage.old`
4. 以同一 PID 重新执行自身（systemd 不会感知到重启），页面等新版本上线后自动刷新，登录状态保留

自动检查默认每天 04:00（服务器本地时间），可在对话框里改成每周或关闭、或切换到预发布通道；自动检查只提示，从不自动安装。状态保存在 `state/update.json`。

命令行也可以操作：

```bash
sudo -u homepage homepage -c /etc/homepage update check
sudo -u homepage homepage -c /etc/homepage update install --yes   # 之后 sudo systemctl restart homepage
sudo -u homepage homepage -c /etc/homepage update rollback --yes  # 换回 homepage.old
```

## 各组件的准备工作

| 组件 | 需要 |
|---|---|
| 局域网 IPv6 | 每台设备运行 ipv6who，其 `allow` 包含 homepage 服务器地址；如设置了 `token`，在 secrets 里加 `ipv6who = "…"` 或给设备单独指定 `token = "<key>"` |
| Vikunja | API token（tasks: read_all + update，projects: read_all）。勾选完成会写回 Vikunja |
| Forgejo | `username`；私有仓库需要 token |
| GitHub | `username`；可选 token 用于贡献日历（GraphQL 需要认证） |
| bookmarkd | 只读集成 token —— **bookmarkd 目前还不支持**，接口约定见 [docs/bookmarkd-read-token.md](docs/bookmarkd-read-token.md) |
| Home Assistant | 长期访问令牌 + `entities` 列表 |

不需要的组件删掉对应配置段即可，卡片会自动重新排版。

## 安全

- 会话：256 位随机令牌，只存 SHA-256 摘要；HttpOnly、`SameSite=Strict`；默认浏览器会话 + 服务端 12 小时，勾选"保持登录"为固定 30 天
- 所有写操作需要 CSRF token，且 `Origin` 必须等于 `public_url`
- 登录限速：每个客户端 1 秒冷却、每分钟最多 5 次失败（通过可信代理识别真实 IP）
- `secrets.toml` 权限过宽时启动会警告；token 不会出现在页面、日志或错误信息里
- 服务探测只判断可达性（接受自签名证书），不会携带任何凭据

## 开发

```bash
dev/setup.sh                        # 生成 dev/config/secrets.toml（用户 chao / 密码 homepage-dev-password）
python3 dev/mock_upstreams.py &     # 模拟 ipv6who、Vikunja、Forgejo、bookmarkd、Home Assistant
cargo run -- -c dev/config          # 后端 :8780
npm --prefix web run dev            # 前端 :5173，代理 /api 到后端
```

```bash
cargo test && cargo clippy --all-targets
npm --prefix web run check
```
