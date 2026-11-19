import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import pytest

from tessh.runner import new_namespace, run_source


@pytest.fixture
def run(capfd):
    """Run tessh source; returns (namespace, stdout)."""
    def go(src, ns=None):
        ns = ns if ns is not None else new_namespace()
        run_source(src, ns)
        out = capfd.readouterr().out
        return ns, out
    return go
