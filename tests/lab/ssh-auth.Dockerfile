ARG LAB_BASE_IMAGE=alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce
FROM ${LAB_BASE_IMAGE}
RUN apk add --no-cache openssh-server \
    && adduser -D -s /bin/sh lab \
    && passwd -d lab \
    && mkdir -p /run/sshd /lab
ENTRYPOINT ["/usr/sbin/sshd", "-D", "-e", "-f", "/lab/sshd_config"]
