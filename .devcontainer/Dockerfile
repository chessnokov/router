ARG RUST_VERSION
FROM docker.io/library/rust:${RUST_VERSION}-alpine

RUN rustup component add rust-src clippy rustfmt

RUN apk update && apk add --no-cache musl-dev 

# VScode deps
ARG INSTALL_ZSH="true"
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
COPY library-scripts/*.sh library-scripts/*.env /tmp/library-scripts/
RUN ash /tmp/library-scripts/common-alpine.sh "${INSTALL_ZSH}" "${USERNAME}" "${USER_UID}" "${USER_GID}" \
    && rm -rf /tmp/library-scripts