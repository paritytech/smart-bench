# Stage 1: builder
FROM docker.io/paritytech/ci-linux:production as builder

COPY . /smart-bench
WORKDIR /smart-bench
RUN cargo build --locked --release


# Stage 2: smart-bench
FROM docker.io/library/ubuntu:20.04
LABEL description="Multistage Docker image for smart-bench"
ARG DOCKERFILE_DIR=launch

RUN apt-get update \
        && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
	  libssl1.1=1.1.1f-1ubuntu2.19 \
	  netcat=1.206-1ubuntu1 \
        && apt-get clean \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder /smart-bench/target/release/smart-bench /usr/local/bin

ENV CONTRACTS_DIR /usr/local/etc/contracts
ENV ZOMBIENET_CONFIGS /usr/local/etc/configs
ENV BINARIES_DIR /usr/local/bin

COPY $DOCKERFILE_DIR/bin/* $BINARIES_DIR/
COPY $DOCKERFILE_DIR/../contracts $CONTRACTS_DIR
COPY $DOCKERFILE_DIR/configs/* $ZOMBIENET_CONFIGS/
COPY $DOCKERFILE_DIR/entrypoint.sh /entrypoint.sh

ENTRYPOINT [ "./entrypoint.sh" ]
