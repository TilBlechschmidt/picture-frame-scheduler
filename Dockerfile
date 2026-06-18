FROM alpine

RUN apk --no-cache add ca-certificates

COPY target/x86_64-unknown-linux-musl/release/picture-frame-scheduler /pfs

CMD /pfs
