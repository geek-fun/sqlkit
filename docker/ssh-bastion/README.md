# SSH Bastion Test Keys

Test keys for the SSH tunnel test stack (`docker-compose-ssh-tunnel.yml`).
Pattern mirrors DocKit's `event-search/docker/ssh-bastion/` setup.

## Files

| File | Purpose | Git Tracked |
|------|---------|-------------|
| `test_key` | ED25519 private key (no passphrase) | ❌ gitignored |
| `test_key.pub` | ED25519 public key | ✅ committed |
| `test_key.pem` | RSA-2048 PEM private key (no passphrase) | ❌ gitignored |
| `test_key.pem.pub` | RSA-2048 PEM public key | ✅ committed |
| `test_key_with_passphrase` | ED25519 private key (**passphrase: `testphrase`**) | ❌ gitignored |
| `test_key_with_passphrase.pub` | matching public key | ✅ committed |
| `authorized_keys` | all three public keys for the bastion container | ✅ committed |

## Start the stack

```bash
docker compose -f docker-compose-ssh-tunnel.yml up -d
# Oracle initializes for 1-3 minutes on first run
```

## Verify the bastion directly

```bash
ssh -i docker/ssh-bastion/test_key -p 2223 tester@localhost echo ok
# password auth also works: tester / testpass
```

## Databases behind the bastion (internal network only)

| Database | Host (as seen from the bastion) | Port | Credentials | Database/Service |
|---|---|---|---|---|
| PostgreSQL 17 | `pg-test` | 5432 | `tester` / `testpass` | `testdb` |
| MySQL 8.4 | `mysql-test` | 3306 | `tester` / `testpass` (root: `testpass`) | `testdb` |
| SQL Server 2022 | `sqlserver-test` | 1433 | `sa` / `TestPass!2024` | `master` |
| Oracle 23ai Free | `oracle-test` | 1521 | `sqlkit` / `testpass` | service `FREEPDB1` |

## SqlKit connection settings

**SSH Tunnel** (Advanced section, or an SSH profile):
- Host: `localhost`, Port: `2223`, Username: `tester`
- Auth: password `testpass` — or Private Key `./docker/ssh-bastion/test_key`
  (ED25519), `./docker/ssh-bastion/test_key.pem` (RSA PEM), or
  `./docker/ssh-bastion/test_key_with_passphrase` (passphrase `testphrase`)

**Database target** (host/port stay internal — the tunnel reaches them):
- PostgreSQL: `pg-test:5432`, tester / testpass, database `testdb`
- MySQL: `mysql-test:3306`, tester / testpass, database `testdb`
- SQL Server: `sqlserver-test:1433`, sa / TestPass!2024
- Oracle: `oracle-test:1521`, service `FREEPDB1`, sqlkit / testpass

⚠️ The database hosts are NOT `localhost` — inside the tunnel they resolve
from the bastion's network (`pg-test`, `mysql-test`, …).
