-- Smoke test: imports a Telltale type to verify the dependency chain resolves.
import SessionTypes.Global

def main : IO Unit :=
  IO.println "jacquard verification: telltale dependency ok"
