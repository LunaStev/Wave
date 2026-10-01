# SPDX-License-Identifier: MPL-2.0
"""Compose target gates without duplicating the policies of focused validators."""

import sys
from tools.ci.common import main

# Validation builds its inputs, then runs the same policy/source/artifact gates
# as CI. Full backend and runtime regression suites belong to tools.ci.test.
PHASES = {"prerequisites", "build", "validation", "artifacts"}


def commands(runner, procedure):
    """A gate succeeds only if every focused validator returns success."""
    for command in procedure["commands"]:
        runner.run(command)


if __name__ == "__main__":
    sys.exit(main("validate"))
