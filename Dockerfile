# Builder stage
FROM rust:1-bookworm as builder

# Install GStreamer development libraries
RUN apt-get update && apt-get install -y \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    libgstrtspserver-1.0-dev gstreamer1.0-rtsp \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install necessary system dependencies (openssl/ca-certificates) and GStreamer runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    tzdata \
    libgstreamer1.0-0 \
    libgstrtspserver-1.0-0 \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-tools \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/ha-sensor-mjpeg /app/ha-sensor-mjpeg

# Copy assets
COPY --from=builder /app/assets /app/assets

# Set env vars (defaults)
ENV PORT=8080

EXPOSE 8080

CMD ["./ha-sensor-mjpeg"]
