FROM rust:1.57 as builder
WORKDIR /tispace
COPY . .
RUN make release

FROM debian:11
RUN apt update && apt install ca-certificates -y && apt clean
COPY --from=builder /tispace/target/release/server /tispace-server
EXPOSE 8080
CMD ["/tispace-server"]
