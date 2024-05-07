set -e
set -x

apt update

# Docker
apt-get install -y \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo \
  "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io

# Signal
apt install -y git maven
mkdir -p /opt/signal
cd /opt/signal
git clone https://github.com/signalapp/Signal-Server.git
cd Signal-Server
git checkout v4.97
mvn -DskipTests package

# Containers
docker run -d --restart unless-stopped --name accountdb -e "POSTGRES_PASSWORD=postgres" -e "POSTGRES_DB=accountdb" -p 5432:5432 postgres:11
docker run -d --restart unless-stopped --name abusedb -e "POSTGRES_PASSWORD=postgres" -e "POSTGRES_DB=abusedb" -p 5433:5432 postgres:11
docker run -d --restart unless-stopped --name messagedb -e "POSTGRES_PASSWORD=postgres" -e "POSTGRES_DB=messagedb" -p 5434:5432 postgres:11
docker run -d --restart unless-stopped --name redis -e "IP=0.0.0.0" -p 7000-7005:7000-7005 grokzen/redis-cluster:latest
docker run -d --restart unless-stopped --name nginx --net="host" -v /etc/nocloud/context/nginx.conf:/etc/nginx/nginx.conf:ro nginx

# Database
cd /opt/signal/Signal-Server/service/target
java -jar TextSecureServer-4.97.jar accountdb migrate /etc/nocloud/context/config.yml
java -jar TextSecureServer-4.97.jar abusedb migrate /etc/nocloud/context/config.yml
java -jar TextSecureServer-4.97.jar messagedb migrate /etc/nocloud/context/config.yml

# Service
service="[Unit]
Description=Signal service
[Service]
ExecStart=/usr/bin/java -jar -Ddw.logging.level=ERROR /opt/signal/Signal-Server/service/target/TextSecureServer-4.97.jar server /etc/nocloud/context/config.yml
WorkingDirectory=/opt/signal/Signal-Server/service/target/
Type=simple
Restart=on-failure
RestartSec=10
[Install]
WantedBy=multi-user.target
"
echo "$service" > /etc/systemd/system/signal.service
systemctl daemon-reload
systemctl enable signal.service
systemctl start signal.service

echo "DONE!"
