# syntax=docker/dockerfile:1

###############################################################################
# Stage 1 — builder: Rust → WebAssembly, then React/Vite bundle → web/dist
###############################################################################
FROM rust:1-bookworm AS builder

# --- Node.js 20 (needed for Vite / tsc) ---
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates gnupg \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# --- Rust wasm32 target + wasm-pack ---
RUN rustup target add wasm32-unknown-unknown \
    && curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install frontend deps first (cacheable layer — changes only when lockfile changes)
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci

# Copy the rest of the source tree (.dockerignore keeps the context small)
WORKDIR /app
COPY . .

# Build WASM (prebuild) + type-check + Vite bundle → /app/web/dist
WORKDIR /app/web
RUN npm run build

###############################################################################
# Stage 2 — runtime: serve the static bundle with nginx
###############################################################################
FROM nginx:1.27-alpine AS runtime

COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/web/dist /usr/share/nginx/html

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1/ >/dev/null 2>&1 || exit 1

CMD ["nginx", "-g", "daemon off;"]
