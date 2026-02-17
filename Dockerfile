# Minimal container image for weft (clawft CLI)
# Built from static musl binary - no runtime dependencies needed
FROM scratch

COPY docker-build/weft-linux-x86_64 /usr/local/bin/weft

# Default config directory
VOLUME ["/root/.clawft"]

ENTRYPOINT ["/usr/local/bin/weft"]
CMD ["gateway"]
