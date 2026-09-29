ARG LAB_BASE_IMAGE=alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce
FROM ${LAB_BASE_IMAGE}
RUN apk add --no-cache openssh-server docker-cli docker-engine python3 sudo \
    && adduser -D -s /bin/sh lab \
    && addgroup lab docker \
    && passwd -d lab \
    && mkdir -p /run/sshd /lab
# Default CLI remains plugin-absent for prior checkpoints. Explicit lab wrapper enables it.
RUN apk add --no-cache docker-cli-compose \
    && mkdir -p /opt/fixture/compose-plugins \
    && for directory in /usr/libexec/docker/cli-plugins /usr/lib/docker/cli-plugins; do \
         if [ -f "$directory/docker-compose" ]; then mv "$directory/docker-compose" /opt/fixture/compose-plugins/docker-compose; fi; \
       done \
    && test -x /opt/fixture/compose-plugins/docker-compose
COPY docker_compose_enabled.sh /opt/fixture/docker-with-compose
RUN chmod 0755 /opt/fixture/docker-with-compose
COPY docker_volume_read.py /opt/fixture/docker-volume-read
RUN chmod 0755 /opt/fixture/docker-volume-read
COPY docker_probe_fixture.py /opt/fixture/docker_probe_fixture.py
COPY docker_list_change.sh /opt/fixture/docker-list-change
COPY docker_list_hang.sh /opt/fixture/docker-list-hang
COPY docker_probe_fault.sh /opt/fixture/docker_probe_fault.sh
RUN mkdir -p /opt/fixture \
    && printf 'lab ALL=(root) ALL\n' > /etc/sudoers.d/lab \
    && chmod 0440 /etc/sudoers.d/lab \
    && chmod 0755 /opt/fixture/docker_probe_fault.sh /opt/fixture/docker-list-hang /opt/fixture/docker-list-change \
    && ln -s docker_probe_fault.sh /opt/fixture/docker-flood \
    && ln -s docker_probe_fault.sh /opt/fixture/docker-hang \
    && for scenario in rootless stopped denied windows; do \
         printf '#!/bin/sh\nexec env DOCKER_CONFIG=/tmp/probe-docker-config DOCKER_HOST=unix:///tmp/probe-%s.sock /usr/bin/docker "$@"\n' "$scenario" > "/opt/fixture/docker-$scenario"; \
         chmod 0755 "/opt/fixture/docker-$scenario"; \
       done
ENTRYPOINT ["/usr/sbin/sshd", "-D", "-e", "-f", "/lab/sshd_config"]
