ARG BASE_IMAGE=ghcr.io/meta-pytorch/openenv-base:latest
ARG INFESTATION_COMMIT=fd0e33e99c4fb9db9106ad14d24d50626a818271

FROM rust:1.91-bookworm AS solver-builder
ARG INFESTATION_COMMIT

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libasound2-dev \
    libgl1-mesa-dev \
    libudev-dev \
    libxi-dev \
    libx11-dev \
    pkg-config \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src/infestation
RUN curl -fsSL "https://github.com/npow/infestation/archive/${INFESTATION_COMMIT}.tar.gz" \
    | tar -xz --strip-components=1
RUN cargo build --release -p solver

FROM ${BASE_IMAGE} AS python-builder

WORKDIR /app
COPY . /app/env
WORKDIR /app/env

RUN --mount=type=cache,target=/root/.cache/uv \
    if [ -f uv.lock ]; then \
        uv sync --frozen --no-install-project --no-editable; \
    else \
        uv sync --no-install-project --no-editable; \
    fi

RUN --mount=type=cache,target=/root/.cache/uv \
    if [ -f uv.lock ]; then \
        uv sync --frozen --no-editable; \
    else \
        uv sync --no-editable; \
    fi

FROM ${BASE_IMAGE}

WORKDIR /app
COPY --from=python-builder /app/env/.venv /app/.venv
COPY --from=python-builder /app/env /app/env
COPY --from=solver-builder /src/infestation/target/release/solver /app/infestation/solver
COPY --from=solver-builder /src/infestation/levels /app/infestation/levels

ENV PATH="/app/.venv/bin:$PATH"
ENV PYTHONPATH="/app/env:$PYTHONPATH"
ENV INFESTATION_SOLVER="/app/infestation/solver"
ENV INFESTATION_LEVELS_DIR="/app/infestation/levels"

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')" || exit 1

CMD ["sh", "-c", "cd /app/env && uvicorn server.app:app --host 0.0.0.0 --port 8000"]
