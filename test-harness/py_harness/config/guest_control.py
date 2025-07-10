import subprocess

def ssh_command(cmd: str, ssh_key: str, hostname: str) -> subprocess.CompletedProcess:
    return subprocess.run(["ssh", "-i", ssh_key,
                          "-o", "StrictHostKeyChecking no",
                          "-o", "UserKnownHostsFile /dev/null",
                          hostname,
                          cmd,
                          ])
