#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if [ -n "${IN_NIX_SHELL:-}" ] && [ -n "${TOOLKIT_ROOT:-}" ] && command -v toolkit-xtask >/dev/null 2>&1; then
  exec "$@"
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

exec nix develop "$toolkit_flake_ref" --command "$@"
