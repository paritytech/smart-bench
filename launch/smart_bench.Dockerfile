# Stage 1: builder
FROM docker.io/paritytech/ci-unified:bullseye-1.74.0 as builder

COPY . /smart-bench
WORKDIR /smart-bench
RUN cargo install --root /usr/local/ --locked --path . \
	&& cargo clean \
	&& rm -rf $CARGO_HOME/registry \
	&& rm -rf $CARGO_HOME/git

# Stage 2: smart-bench
FROM docker.io/library/ubuntu:20.04
LABEL description="Multistage Docker image for smart-bench"
ARG DOCKERFILE_DIR=launch

RUN apt-get update \
        && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
	  libssl1.1=1.1.1f-1ubuntu2.20 \
	  netcat=1.206-1ubuntu1 \
        && apt-get clean \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/smart-bench /usr/local/bin

ENV CONTRACTS_DIR /usr/local/smart-bench/contracts
ENV CONFIGS_DIR /usr/local/smart-bench/config
ENV BINARIES_DIR /usr/local/smart-bench/bin

COPY $DOCKERFILE_DIR/bin/* $BINARIES_DIR/
COPY $DOCKERFILE_DIR/../contracts $CONTRACTS_DIR
COPY $DOCKERFILE_DIR/configs/* $CONFIGS_DIR/
COPY $DOCKERFILE_DIR/entrypoint.sh /entrypoint.sh

ENTRYPOINT [ "./entrypoint.sh" ]
