#!/bin/sh
# Runs ONLY inside the disposable QEMU guest, from its private NoCloud seed.
set -eu
apk add --no-cache docker docker-cli-compose openssh python3
rc-service cgroups start
rc-service docker start
adduser -D -s /bin/sh lab
addgroup lab docker
passwd -d lab
passwd -d root
chmod 755 /opt/containerdesk
chmod 644 /opt/containerdesk/authorized_keys
chmod 600 /opt/containerdesk/*-key
for role in direct jump private; do
    /usr/sbin/sshd -f "/opt/containerdesk/sshd-$role.conf"
done
docker pull 'alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce'
docker run -d --name owned-live --network none --read-only --cap-drop ALL --user 1000:1000 \
    --label dev.containerdesk.lab=049 --env TOKEN=SEEDED_049_ENV \
    'alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce' \
    /bin/sh -c 'while :; do echo CD049_LOG; sleep 1; done'
docker run -d --name untouched --network none --read-only --cap-drop ALL \
    --label dev.containerdesk.lab=049 \
    'alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce' sleep 3600
docker compose --project-name owned049 --file "/opt/owned project's/compose.yml" up -d
# This is setup in a wholly disposable daemon, never an application operation.
touch /opt/containerdesk/ready
