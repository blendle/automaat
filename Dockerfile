FROM alpine
EXPOSE 80

ENV DATABASE_URL postgres://postgres@localhost
ENV SERVER_BIND 0.0.0.0:80
ENV SERVER_ROOT /public

COPY ./src/web-client/static /public
COPY ./target/x86_64-unknown-linux-musl/release/automaat-server /automaat

ENTRYPOINT ["/automaat"]
CMD ["server"]
