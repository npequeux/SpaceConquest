"""Entry point for Stellar Dominion.

Run from this directory with the project environment:

    uv run python main.py

or, with pygame-ce installed in your own interpreter:

    python main.py
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from stellar_dominion.ui import main  # noqa: E402

if __name__ == "__main__":
    main()
