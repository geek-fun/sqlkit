# SSH Tunnel 测试指南（sqlkit）

配套测试栈：`docker-compose-ssh-tunnel.yml`（本仓库根目录）。
架构与实现见 [ssh-tunnel-architecture.md](ssh-tunnel-architecture.md)。

## 启动 / 停止

```bash
docker compose -f docker-compose-ssh-tunnel.yml up -d
docker compose -f docker-compose-ssh-tunnel.yml down   # 数据卷保留
```

- 堡垒机对宿主机暴露 **2223** 端口（DocKit 的测试栈用 2222，两者可并行运行）
- 四个数据库只在内部网络，**不对宿主机暴露**
- Oracle 首次初始化需要 1-3 分钟，其余容器 1 分钟内 healthy

## 已验证状态（2026-10-09）

- 容器全部 healthy
- ED25519 密钥登录堡垒机 ✓
- 从堡垒机 `nc` 探测四个库端口全部可达 ✓

## 测试密钥（三把，覆盖三种认证路径）

私钥已 gitignore，仅存在于本地 `docker/ssh-bastion/`：

| 私钥 | 认证路径 |
|---|---|
| `docker/ssh-bastion/test_key` | ED25519 |
| `docker/ssh-bastion/test_key.pem` | RSA PEM |
| `docker/ssh-bastion/test_key_with_passphrase` | ED25519 + 口令（口令：`testphrase`）|

## 数据库凭据

| 数据库 | 堡垒机内侧地址 | 凭据 |
|---|---|---|
| PostgreSQL 17 | `pg-test:5432` | tester / testpass / testdb |
| MySQL 8.4 | `mysql-test:3306` | tester / testpass（root 同密）|
| SQL Server 2022 | `sqlserver-test:1433` | sa / TestPass!2024 |
| Oracle 23ai Free | `oracle-test:1521` | sqlkit / testpass / 服务名 FREEPDB1 |

## 在 SqlKit 里配置

连接对话框 → **Advanced Configuration → SSH Tunnel**：

- Host：`localhost`
- Port：`2223`
- 用户：`tester`
- 认证：密码 `testpass`，或三把私钥任选（路径见上表）

⚠️ **数据库主机填内部名**（`pg-test` / `mysql-test` / `sqlserver-test` / `oracle-test`），
不是 localhost —— 隧道是从堡垒机网络去连数据库的。

也可以改用 **SSH Profile**：连接页顶部的 SSH Profile 卡片区新建档案
（同样的堡垒机配置），连接对话框选择 "SSH profiles" 来源并按顺序添加跳板
（支持多跳，最后一跳转发到数据库）。

## 排错

- 端口探测请用 `echo | nc -w 3 <host> <port>`（Alpine bash 不支持
  `/dev/tcp`，`nc -z` 在部分变体上返回假阴性）
- Oracle 未就绪：`docker logs sqlkit-ssh-oracle` 看初始化进度
- 主机密钥告警：TOFU 首连会钉扎指纹；服务器密钥变更后用
  `unpin_ssh_host` 命令清除（或删除
  `~/Library/Application Support/.../ssh_known_hosts.json` 中对应条目）
