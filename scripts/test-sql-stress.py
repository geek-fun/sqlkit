#!/usr/bin/env python3
"""Run docs/sql-test through SQLKit's real adapters against disposable databases."""
import argparse
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
ENGINES = {"POSTGRES": ("pg-test", 5432), "MYSQL": ("mysql-test", 3306),
           "SQLSERVER": ("sqlserver-test", 1433), "ORACLE": ("oracle-test", 1521)}


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ssh-key", type=Path, default=ROOT.parent / "event-search/docker/ssh-bastion-sqlkit/test_key")
    parser.add_argument("--ssh-host", default="localhost")
    parser.add_argument("--ssh-port", type=int, default=2223)
    parser.add_argument("--ssh-user", default="tester")
    parser.add_argument("--direct", action="store_true", help="Use database environment variables without starting SSH")
    parser.add_argument("--engine", choices=[name.lower() for name in ENGINES])
    parser.add_argument("--java-home", type=Path, help="JDK 25+ used to rebuild the bridge")
    parser.add_argument("--skip-bridge-build", action="store_true")
    parser.add_argument("--log", type=Path, default=Path(tempfile.gettempdir()) / "sqlkit-stress.log")
    args = parser.parse_args()
    env = os.environ.copy()
    env["SQLKIT_STRESS_RESET"] = "1"
    if args.java_home:
        env["JAVA_HOME"] = str(args.java_home.resolve())
    if args.engine in (None, "oracle"):
        if not args.skip_bridge_build:
            if not shutil.which("mvn"):
                parser.error("Maven and JDK 25+ are required to build the JDBC bridge")
            subprocess.run(["mvn", "-q", "-f", str(ROOT / "jdbc-bridge/pom.xml"), "package"], env=env, check=True)
        jar = ROOT / "jdbc-bridge/target/jdbc-bridge-1.0.0.jar"
        if not jar.is_file():
            parser.error("Build jdbc-bridge first; no local bridge JAR found")
        env["SQLKIT_JDBC_BRIDGE_JAR"] = str(jar)
    tunnel = None
    try:
        if not args.direct:
            if not args.ssh_key.is_file():
                parser.error("SSH key not found; provide --ssh-key or use --direct")
            command = ["ssh", "-N", "-o", "BatchMode=yes", "-o", "ExitOnForwardFailure=yes",
                       "-o", "StrictHostKeyChecking=accept-new", "-i", str(args.ssh_key),
                       "-p", str(args.ssh_port)]
            ports = []
            for name, (host, remote_port) in ENGINES.items():
                port = free_port()
                ports.append(port)
                env[f"{name}_HOST"] = "127.0.0.1"
                env[f"{name}_PORT"] = str(port)
                command.extend(["-L", f"127.0.0.1:{port}:{host}:{remote_port}"])
            command.append(f"{args.ssh_user}@{args.ssh_host}")
            tunnel = subprocess.Popen(command)
            deadline = time.monotonic() + 10
            while True:
                if tunnel.poll() is not None:
                    raise RuntimeError("SSH tunnel exited before startup")
                try:
                    with socket.create_connection(("127.0.0.1", ports[0]), timeout=0.2):
                        break
                except OSError:
                    if time.monotonic() >= deadline:
                        raise RuntimeError("SSH tunnel did not become ready")
                    time.sleep(0.1)
        command = ["cargo", "test", "--manifest-path", str(ROOT / "src-tauri/Cargo.toml"),
                   "--test", "sql_stress_integration"]
        if args.engine:
            command.append(f"{args.engine}_stress_suite")
        command.extend(["--", "--ignored", "--nocapture"])
        print(f"Recreating docs/sql-test fixtures through SQLKit; log: {args.log}", flush=True)
        with args.log.open("w") as log:
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
        print("\n".join(args.log.read_text().splitlines()[-30:]))
        return result.returncode
    finally:
        if tunnel is not None:
            tunnel.terminate()
            try:
                tunnel.wait(timeout=5)
            except subprocess.TimeoutExpired:
                tunnel.kill()
                tunnel.wait()


if __name__ == "__main__":
    sys.exit(main())
