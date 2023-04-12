FROM scratch
COPY image_uploader /
RUN chmod +x /image_uploader
CMD ["/image_uploader"]
LABEL org.opencontainers.image.source=https://github.com/MaybeNotASpy/REPO