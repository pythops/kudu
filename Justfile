default:
    @just --list

run:
    cargo run

clean:
    rm -rf $XDG_RUNTIME_DIR/kudu
    rm -rf ~/.local/share/kudu

profile:
    CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph
