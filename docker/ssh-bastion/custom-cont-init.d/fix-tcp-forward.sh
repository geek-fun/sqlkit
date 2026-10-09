#!/bin/bash
sed -i 's/AllowTcpForwarding no/AllowTcpForwarding yes/' /config/sshd/sshd_config
echo "TCP forwarding enabled in /config/sshd/sshd_config"
