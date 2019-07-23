FROM ekidd/rust-musl-builder:latest as builder
ADD . ./
RUN curl --location -O https://github.com/sass/dart-sass/releases/download/1.22.7/dart-sass-1.22.7-linux-x64.tar.gz && \
    tar xvzf dart-sass-1.22.7-linux-x64.tar.gz
ENV PATH="/home/rust/src/dart-sass:${PATH}"
RUN sudo chown -R rust:rust /home/rust
RUN cargo build --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/server /usr/local/bin/kachiclash
EXPOSE 8000
CMD ["/usr/local/bin/kachiclash"]
