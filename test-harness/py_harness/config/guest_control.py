import subprocess
from subprocess import Popen


def ssh_command(cmd: str, ssh_key: str, hostname: str, capture_output: bool = False) -> subprocess.CompletedProcess:
    return subprocess.run(["ssh", "-i", ssh_key,
                          "-o", "StrictHostKeyChecking no",
                          "-o", "UserKnownHostsFile /dev/null",
                          hostname,
                          cmd,
                          ],
                          capture_output=capture_output,
                          )

def long_running_ssh_command(cmd: str, ssh_key: str, hostname: str) -> Popen[str]:
    return subprocess.Popen(["ssh", "-i", ssh_key,
                             "-o", "StrictHostKeyChecking no",
                             "-o", "UserKnownHostsFile /dev/null",
                             hostname,
                             cmd,
                             ],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            bufsize=1
                            )
