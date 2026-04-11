#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

sanitize_path() {
  perl -e '
    my $path = $ENV{PATH} // q();
    my $home = $ENV{HOME} // q();
    my $cargo_bin = $home eq q() ? q() : "$home/.cargo/bin";
    my @parts = grep { $_ ne q() && $_ ne $cargo_bin } split(/:/, $path, -1);
    print join(":", @parts);
  '
}

sanitized_path="$(sanitize_path)"

run_sanitized() {
  env \
    -u CARGO \
    -u RUSTC \
    -u RUSTDOC \
    -u RUSTUP_TOOLCHAIN \
    PATH="$sanitized_path" \
    "$@"
}

if [ -n "${IN_NIX_SHELL:-}" ] && [ -n "${TOOLKIT_ROOT:-}" ] && command -v toolkit-xtask >/dev/null 2>&1; then
  exec env \
    -u CARGO \
    -u RUSTC \
    -u RUSTDOC \
    -u RUSTUP_TOOLCHAIN \
    PATH="$sanitized_path" \
    "$@"
fi

toolkit_flake_ref="$(
  perl -MJSON::PP -e '
    my $path = shift;
    open my $fh, "<", $path or die "failed to open $path: $!";
    local $/;
    my $lock = decode_json(<$fh>);
    my $node = $lock->{nodes}{toolkit}{locked}
      or die "missing toolkit lock entry\n";
    die "unsupported toolkit lock type: " . ($node->{type} // q()) . "\n"
      unless ($node->{type} // q()) eq "github";
    my $ref = "github:$node->{owner}/$node->{repo}/$node->{rev}";
    $ref .= "?narHash=$node->{narHash}" if exists $node->{narHash};
    print $ref;
  ' "$repo_root/flake.lock"
)"

run_sanitized nix develop "$toolkit_flake_ref" --command "$@"
