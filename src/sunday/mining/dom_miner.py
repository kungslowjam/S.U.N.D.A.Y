"""Python interface for High-speed DOM Mining (Rust-backed)."""

from __future__ import annotations
import logging
from typing import Optional
from sunday._rust_bridge import get_rust_module

logger = logging.getLogger("sunday.mining.dom_miner")

class DOMMiner:
    """A high-performance DOM mining utility backed by Rust."""
    
    def __init__(self):
        self._rust = get_rust_module()
        self._inner = None
        if hasattr(self._rust, "NativeMiner"):
            self._inner = self._rust.NativeMiner()
        else:
            logger.warning("NativeMiner not found in sunday_rust module. Rust acceleration disabled.")

    def mine(self, html: str) -> str:
        """
        Extract key information from HTML using Rust's high-speed parser.
        Returns a formatted string suitable for LLM context.
        """
        if self._inner:
            try:
                return self._inner.mine_html(html)
            except Exception as e:
                logger.error(f"Rust DOM mining failed: {e}")
                return self._fallback_mine(html)
        return self._fallback_mine(html)

    def _fallback_mine(self, html: str) -> str:
        """Slow Python fallback if Rust is unavailable."""
        # Minimal implementation for fallback
        return f"[Fallback] HTML Length: {len(html)} characters. (Please ensure Rust backend is compiled)"

def smart_scrape(html: str) -> str:
    """One-shot utility for high-speed scraping."""
    return DOMMiner().mine(html)
