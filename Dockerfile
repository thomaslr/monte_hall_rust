# ===========================
# Stage 1: Build the WASM app
# ===========================
FROM rust:latest AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm target and tools
RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked trunk
RUN cargo install --locked wasm-bindgen-cli --version 0.2.114

WORKDIR /app

# Copy only manifests + dummy source for dependency caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --target wasm32-unknown-unknown --release 2>/dev/null || true
RUN rm -rf src

# Copy full source
COPY . .

# Build with trunk for production
RUN trunk build --release

# ===========================
# Stage 2: Serve with nginx
# ===========================
FROM nginx:alpine

# Copy built dist to nginx html
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
