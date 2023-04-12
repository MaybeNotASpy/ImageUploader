FROM scratch
COPY image_uploader /
ENTRYPOINT ["/image_uploader"]
LABEL org.opencontainers.image.source=https://github.com/MaybeNotASpy/REPO