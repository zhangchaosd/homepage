# Homepage

自托管服务的统一入口：一个单文件二进制 + 一个配置目录。

- **服务启动台**：一键打开 Forgejo、Vikunja、bookmarkd、Obsidian Web Gateway、Home Assistant…，每个服务带在线状态（服务端探测局域网地址）
- **局域网 IPv6**：通过各设备上的 [ipv6who](https://github.com/zhangchaosd/ipv6who) 显示当前全局 IPv6 地址，点击即可复制
- **今日待办**：Vikunja 中已逾期、今天和未来几天的任务，可直接勾选完成
- **最近提交**：Forgejo 动态里的提交 + 近半年贡献热力图
- **GitHub**：资料、最近推送的仓库，配置 token 后显示贡献日历
- **置顶书签**：bookmarkd 中置顶的书签（需要只读 token，见下文）
- **家**：Home Assistant 中指定实体的状态
- **收件箱**：在主页上直接投递文字和链接到 [跨设备收件箱](https://github.com/zhangchaosd/cross-device-inbox)（inboxd），正文中的网址自动识别为链接；令牌可读时同时显示最近收件
- `/` 或 `⌘K` 搜索服务和书签，回车打开第一个结果
- **在线更新**：每天自动检查 GitHub Releases，右上角提示新版本，点"安装并重启"即可升级，无需重新部署

登录方式与 obsidian-web-gateway 相同：**用户名 + 密码**（Argon2）和/或 **Passkey**（复用 bookmarkd 的 `auth.db`）。整站都需要登录，未登录只能看到登录页。

## 工作方式

```
浏览器 ──HTTPS──> Caddy（网关机）──HTTP──> homepage（局域网）
                                            ├─ 后台定时拉取（各自间隔，失败保留旧数据）
                                            │   ipv6who × N / Vikunja / Forgejo / GitHub / bookmarkd / Home Assistant / inboxd
                                            └─ 内嵌前端（Svelte），所有 token 只在服务端
```

浏览器只和 homepage 通信：token 不会下发，GitHub 头像也由服务端内联，CSP 保持 `default-src 'self'`。页面每 30 秒刷新一次缓存数据，标签页不可见时暂停。

## 安装

### Linux（systemd 用户服务）

所有文件都放在 `~/homepage` 目录下，以当前用户身份运行，不需要 sudo：

```sh
mkdir -p ~/homepage && cd ~/homepage
curl -fsSL https://github.com/zhangchaosd/homepage/releases/latest/download/homepage-linux-$(uname -m).tar.gz | tar -xz --strip-components=1
cp -n examples/config/config.toml config.toml
(umask 077 && ./homepage hash-password > secrets.toml)
${EDITOR:-vi} config.toml secrets.toml
./homepage check-config
cat > homepage.service <<'EOF'
[Unit]
Description=Homepage — personal start page
After=network-online.target

[Service]
Type=simple
WorkingDirectory=%h/homepage
ExecStart=%h/homepage/homepage
Restart=on-failure
RestartSec=5
UMask=0077
StandardOutput=append:%h/homepage/homepage.log
StandardError=append:%h/homepage/homepage.log

[Install]
WantedBy=default.target
EOF
systemctl --user enable --now ~/homepage/homepage.service
curl -s --retry 5 --retry-connrefused http://127.0.0.1:8780/healthz
```

`hash-password` 会提示输入两次登录密码，`secrets.toml` 里先只有密码哈希。在编辑器里改好 `config.toml` 中各服务的地址，并参照 `examples/config/secrets.toml.example` 往 `secrets.toml` 加上 token；`check-config` 通过后再启动服务。Linux 版只依赖 glibc 2.28（v0.1.5 起；更早的版本需要 2.39，即 Ubuntu 24.04+、Debian 13+）。

修改配置：编辑 `~/homepage/config.toml` 或 `secrets.toml`，然后执行 `systemctl --user restart homepage`。
日志：`~/homepage/homepage.log`。以后升级在页面右上角一键完成（见[更新](#更新)），不需要重复这些步骤。如需开机后无需登录即自动运行，执行 `loginctl enable-linger`
（部分系统需要 `sudo loginctl enable-linger $USER`）。

卸载：

```sh
systemctl --user disable --now homepage
rm -rf ~/homepage
```

如需加固的系统级服务，可参考 [examples/homepage.service](examples/homepage.service)（程序放在 `/opt/homepage`，配置放在 `/etc/homepage`）。

### 从源码构建

需要 Rust 1.88+ 和 Node 20+（只在构建时需要）。产物是 `target/release/homepage`，复制到 `~/homepage/homepage` 即可代替下载。

```bash
scripts/build.sh
TARGET=x86_64-unknown-linux-gnu scripts/build.sh   # 交叉编译
```

## 配置

```
~/homepage/
├── homepage          # 程序本身，在线更新时原地替换（旧版本保留为 homepage.old）
├── config.toml       # 服务、设备、组件（参考 examples/config/config.toml）
├── secrets.toml      # 密码哈希和各种 token，权限 600
├── auth.db           # 从 bookmarkd 复制的 Passkey 数据库（可选）
├── icons/            # 自定义服务图标（可选）
├── state/            # 运行时写入（会话、更新状态），自动创建
├── homepage.log
└── homepage.service
```

`homepage` 默认把当前目录当作配置目录；从别处运行时用 `-c <目录>` 指定。

### Passkey（复用 bookmarkd）

Passkey 绑定在 RP ID 上（例如 `example.com`），所以主页的 `public_url` 必须是这个域名或它的子域名，例如 `https://home.example.com`。

```sh
# 在 bookmarkd 所在机器上（sqlite3 .backup 保证拿到一致的副本）
sqlite3 /var/lib/bookmarkd/auth.db ".backup /tmp/auth.db"
scp /tmp/auth.db homepage-server:homepage/auth.db
# 在 homepage 所在机器上：取消 config.toml 中 [auth] 里 passkey_db = "auth.db" 的注释
chmod 600 ~/homepage/auth.db
systemctl --user restart homepage
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

**发版**（在 Apple Silicon Mac 上本地完成，不使用 GitHub Actions）：改 `Cargo.toml` 和 `web/package.json` 的 `version` 并提交，然后：

```bash
SMOKE_HOST=zc@192.168.200.100 scripts/release.sh            # 演练：检查、构建四个平台、冒烟测试、打包
SMOKE_HOST=zc@192.168.200.100 scripts/release.sh --publish  # 打 tag、推送、创建 GitHub Release 并核对校验和
```

脚本依次运行前端检查与构建、fmt/clippy/测试，原生构建 macOS arm64、交叉构建 macOS x86_64，用 cargo-zigbuild 构建 Linux x86_64/aarch64（只依赖 glibc 2.28，Debian 12、Ubuntu 22.04 也能运行），打包到 `target/release-X.Y.Z/`，在本机和 `SMOKE_HOST`（可选的 x86_64 Linux 机器，测试在临时目录里进行，不影响已安装的服务）上冒烟测试。需要 Zig 0.14.1（默认 `~/.local/zig-0.14.1`，可用 `ZIG_HOME` 指定）、cargo-zigbuild 和登录好的 `gh`。

文件名不带版本号（如 `homepage-linux-x86_64.tar.gz`），所以安装命令里的 `latest/download` 链接始终指向最新版；更新器会校验新程序报告的版本号。带 `-` 的版本（如 `0.2.0-beta.1`）发布为预发布版。

**升级**（在主页上）：右上角齿轮按钮出现版本号时，点开"版本与更新"→"安装并重启"。服务端会：

1. 从 GitHub Releases 下载对应平台的压缩包，按 `SHA256SUMS.txt` 校验
2. 解出新二进制，运行 `--version` 确认版本正确
3. 原地替换，旧版本保留为 `homepage.old`
4. 以同一 PID 重新执行自身（systemd 不会感知到重启），页面等新版本上线后自动刷新，登录状态保留

自动检查默认每天 04:00（服务器本地时间），可在对话框里改成每周或关闭、或切换到预发布通道；自动检查只提示，从不自动安装。状态保存在 `state/update.json`。

命令行也可以操作：

```bash
cd ~/homepage
./homepage update check
./homepage update install --yes    # 之后 systemctl --user restart homepage
./homepage update rollback --yes   # 换回 homepage.old
```

## 各组件的准备工作

| 组件 | 需要 |
|---|---|
| 局域网 IPv6 | 每台设备运行 ipv6who，其 `allow` 包含 homepage 服务器地址；如设置了 `token`，在 secrets 里加 `ipv6who = "…"` 或给设备单独指定 `token = "<key>"` |
| Vikunja | API token（tasks: read_all + update，projects: read_all）。勾选完成会写回 Vikunja |
| Forgejo | `username`；私有仓库需要 token |
| GitHub | `username`；可选 token 用于贡献日历（GraphQL 需要认证） |
| bookmarkd | v0.7.0+。在 bookmarkd 所在机器上运行 `bookmarkd token create homepage` 生成只读 token。`link` 必须是 bookmarkd 的 `public_url`：bookmarkd 只接受 Host 为该地址的请求，`url` 用局域网地址时主页会自动带上这个 Host |
| Home Assistant | 长期访问令牌 + `entities` 列表 |
| 收件箱（inboxd） | inboxd 0.3.1+：在收件箱「设置 → 令牌」里创建**读写**令牌（默认 100 年有效），写入 `secrets.toml` 的 `inbox`。读写令牌既能投递也能显示最近收件（`limit` 条，默认 6）；只写令牌只能投递。`url` 填局域网地址（inboxd 不校验 Host），`link` 填公开地址。令牌到期前 30 天卡片会提醒 |

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
