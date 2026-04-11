#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

sanitize_path() {
  perl -e '
    my $path = $ENV{PATH} // q();
    my $home = $ENV{HOME} // q();
    my $cargo_home = $ENV{CARGO_HOME} // ($home eq q() ? q() : "$home/.cargo");
    my @drop = grep { $_ ne q() } (
      $home eq q() ? q() : "$home/.cargo/bin",
      $cargo_home eq q() ? q() : "$cargo_home/bin",
    );
    my %drop = map { $_ => 1 } @drop;
    my @parts = grep { $_ ne q() && !$drop{$_} } split(/:/, $path, -1);
    print join(":", @parts);
  '
}

run_sanitized() {
  local sanitized_path
  sanitized_path="$(sanitize_path)"
  env \
    -u CARGO \
    -u RUSTC \
    -u RUSTDOC \
    -u RUSTUP_TOOLCHAIN \
    PATH="$sanitized_path" \
    "$@"
}

if [ "${1:-}" = "--inside-nix" ]; then
  shift
  if [ -z "${IN_NIX_SHELL:-}" ] || [ -z "${TOOLKIT_ROOT:-}" ]; then
    echo "toolkit-shell.sh: --inside-nix requires the toolkit nix shell" >&2
    exit 1
  fi
  run_sanitized "$@"
  exit $?
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

run_sanitized nix develop "$toolkit_flake_ref" --command \
  "$repo_root/scripts/toolkit-shell.sh" --inside-nix "$@"
