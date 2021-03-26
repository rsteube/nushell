FROM rust as nushell-poc
ADD . /nushell
RUN cd nushell && cargo build --release

FROM scratch
COPY --from=nushell-poc /nushell/target/release/nu /usr/local/bin/
