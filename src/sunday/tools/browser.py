"""Browser automation tools — Playwright-based web interaction."""

from __future__ import annotations

import base64
from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec


class _BrowserSession:
    """Manages a shared Playwright browser session (lazy init)."""

    def __init__(self) -> None:
        self._playwright = None
        self._browser = None
        self._page = None

    def _ensure_browser(self) -> None:
        if self._page is not None:
            return
        try:
            from playwright.sync_api import sync_playwright
        except ImportError:
            raise ImportError(
                "playwright not installed. Install with: uv sync --extra browser"
            )
        self._playwright = sync_playwright().start()
        self._browser = self._playwright.chromium.launch(headless=False)
        self._page = self._browser.new_page(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
            viewport={"width": 1280, "height": 720}
        )

    @property
    def page(self):
        self._ensure_browser()
        return self._page

    def close(self) -> None:
        if self._browser:
            self._browser.close()
        if self._playwright:
            self._playwright.stop()
        self._playwright = self._browser = self._page = None


def _capture_metadata(page) -> dict:
    """Capture screenshot and other metadata for visual feedback."""
    try:
        import base64
        import os
        # Capture screenshot for the UI
        screenshot_bytes = page.screenshot()
        b64_data = base64.b64encode(screenshot_bytes).decode("utf-8")
        
        # Also save to last_screenshot.png for convenience
        # Find project root (4 levels up from this file: src/sunday/tools/browser.py)
        root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
        with open(os.path.join(root, "last_screenshot.png"), "wb") as f:
            f.write(screenshot_bytes)
            
        return {"screenshot_base64": b64_data}
    except Exception:
        return {}


_session = _BrowserSession()


# ---------------------------------------------------------------------------
# Tool 1: BrowserNavigateTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_navigate")
class BrowserNavigateTool(BaseTool):
    """Navigate to a URL in the browser."""

    tool_id = "browser_navigate"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_navigate",
            description=(
                "Navigate to a URL in the browser."
                " Returns the page title and text content."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to.",
                    },
                    "wait_for": {
                        "type": "string",
                        "description": (
                            "Wait condition: 'load', 'domcontentloaded',"
                            " or 'networkidle'. Default: 'load'."
                        ),
                    },
                },
                "required": ["url"],
            },
            category="browser",
            required_capabilities=["network:fetch"],
        )

    def execute(self, **params: Any) -> ToolResult:
        url = params.get("url", "")
        if not url:
            return ToolResult(
                tool_name="browser_navigate",
                content="No URL provided.",
                success=False,
            )

        wait_for = params.get("wait_for", "load")
        if wait_for not in ("load", "domcontentloaded", "networkidle"):
            wait_for = "load"

        # SSRF check
        try:
            from sunday.security.ssrf import check_ssrf

            ssrf_error = check_ssrf(url)
            if ssrf_error:
                return ToolResult(
                    tool_name="browser_navigate",
                    content=f"SSRF blocked: {ssrf_error}",
                    success=False,
                )
        except ImportError:
            pass  # ssrf module not available, skip check

        try:
            page = _session.page
            page.goto(url, wait_until=wait_for)
            title = page.title()
            
            # Auto-capture screenshot for visual feedback
            meta = _capture_metadata(page)
            
            return ToolResult(
                tool_name="browser_navigate",
                content=f"Navigated to {url}. Title: {title}",
                success=True,
                metadata={**meta, "url": url, "title": title},
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_navigate",
                content=f"Navigation error: {exc}",
                success=False,
            )


# ---------------------------------------------------------------------------
# Tool 2: BrowserClickTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_click")
class BrowserClickTool(BaseTool):
    """Click an element on the page."""

    tool_id = "browser_click"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_click",
            description=(
                "Click an element on the current page."
                " Use a CSS selector or text content to identify the element."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or text content of the element.",
                    },
                    "by_text": {
                        "type": "boolean",
                        "description": (
                            "If true, click by text content"
                            " instead of CSS selector. Default: false."
                        ),
                    },
                },
                "required": ["selector"],
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        selector = params.get("selector", "")
        if not selector:
            return ToolResult(
                tool_name="browser_click",
                content="No selector provided.",
                success=False,
            )

        by_text = params.get("by_text", False)

        try:
            page = _session.page
            # Wait for element to be visible first
            try:
                page.wait_for_selector(selector, timeout=5000)
            except Exception:
                pass # Continue anyway, maybe it's there
                
            if by_text:
                page.get_by_text(selector).click()
            else:
                page.click(selector)
            # Auto-capture screenshot for visual feedback
            meta = _capture_metadata(page)

            return ToolResult(
                tool_name="browser_click",
                content=f"Clicked element: {selector}",
                success=True,
                metadata={**meta, "selector": selector, "by_text": by_text},
            )
        except ImportError:
            return ToolResult(
                tool_name="browser_click",
                content=(
                    "playwright not installed. Install with: uv sync --extra browser"
                ),
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_click",
                content=f"Click error: {exc}",
                success=False,
            )


# ---------------------------------------------------------------------------
# Tool 3: BrowserTypeTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_type")
class BrowserTypeTool(BaseTool):
    """Type text into a form field."""

    tool_id = "browser_type"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_type",
            description=(
                "Type text into a form field on the current page."
                " Can clear the field first or append to existing content."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of the input field.",
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type into the field.",
                    },
                    "clear": {
                        "type": "boolean",
                        "description": (
                            "If true, clear the field before typing. Default: true."
                        ),
                    },
                },
                "required": ["selector", "text"],
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        selector = params.get("selector", "")
        text = params.get("text", "")

        if not selector:
            return ToolResult(
                tool_name="browser_type",
                content="No selector provided.",
                success=False,
            )
        if not text:
            return ToolResult(
                tool_name="browser_type",
                content="No text provided.",
                success=False,
            )

        clear = params.get("clear", True)

        try:
            page = _session.page
            # Wait for element to be visible first
            try:
                page.wait_for_selector(selector, timeout=5000)
            except Exception:
                pass

            if clear:
                page.fill(selector, text)
            else:
                page.type(selector, text)
            # Auto-capture screenshot for visual feedback
            meta = _capture_metadata(page)

            return ToolResult(
                tool_name="browser_type",
                content=f"Typed text into {selector}: {text}",
                success=True,
                metadata={**meta, "selector": selector, "text": text},
            )
        except ImportError:
            return ToolResult(
                tool_name="browser_type",
                content=(
                    "playwright not installed. Install with: uv sync --extra browser"
                ),
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_type",
                content=f"Type error: {exc}",
                success=False,
            )


# ---------------------------------------------------------------------------
# Tool 4: BrowserScreenshotTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_screenshot")
class BrowserScreenshotTool(BaseTool):
    """Take a screenshot of the current page."""

    tool_id = "browser_screenshot"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_screenshot",
            description=(
                "Take a screenshot of the current browser page."
                " Returns the screenshot as base64-encoded data."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional file path to save the screenshot.",
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": (
                            "If true, capture the full scrollable page. Default: false."
                        ),
                    },
                },
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        import os
        # Find project root (4 levels up from this file: src/sunday/tools/browser.py)
        root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
        default_path = os.path.join(root, "last_screenshot.png")
        path = params.get("path") or default_path
        full_page = params.get("full_page", False)

        try:
            page = _session.page
            screenshot_bytes = page.screenshot(full_page=full_page)

            with open(path, "wb") as f:
                f.write(screenshot_bytes)

            b64_data = base64.b64encode(screenshot_bytes).decode("utf-8")

            description = "Screenshot taken"
            if full_page:
                description += " (full page)"
            if path:
                description += f", saved to {path}"

            return ToolResult(
                tool_name="browser_screenshot",
                content=description,
                success=True,
                metadata={"screenshot_base64": b64_data},
            )
        except ImportError:
            return ToolResult(
                tool_name="browser_screenshot",
                content=(
                    "playwright not installed. Install with: uv sync --extra browser"
                ),
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_screenshot",
                content=f"Screenshot error: {exc}",
                success=False,
            )


# ---------------------------------------------------------------------------
# Tool 5: BrowserExtractTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_extract")
class BrowserExtractTool(BaseTool):
    """Extract content from the current page."""

    tool_id = "browser_extract"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_extract",
            description=(
                "Extract content from the current browser page."
                " Supports extracting text, links, or tables."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": (
                            "CSS selector to extract from. Default: 'body'."
                        ),
                    },
                    "extract_type": {
                        "type": "string",
                        "description": (
                            "Type of extraction: 'text', 'links',"
                            " or 'tables'. Default: 'text'."
                        ),
                    },
                },
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        selector = params.get("selector", "body")
        extract_type = params.get("extract_type", "text")

        if extract_type not in ("text", "links", "tables"):
            return ToolResult(
                tool_name="browser_extract",
                content=(
                    f"Invalid extract_type: '{extract_type}'."
                    " Must be 'text', 'links', or 'tables'."
                ),
                success=False,
            )

        try:
            page = _session.page

            if extract_type == "text":
                content = page.inner_text(selector)
                if len(content) > 10000:
                    content = content[:10000] + "\n\n[Content truncated]"
                return ToolResult(
                    tool_name="browser_extract",
                    content=content,
                    success=True,
                    metadata={"selector": selector, "extract_type": extract_type},
                )

            elif extract_type == "links":
                links = page.eval_on_selector_all(
                    f"{selector} a[href]",
                    """elements => elements.map(el => ({
                        href: el.href,
                        text: el.innerText.trim()
                    }))""",
                )
                lines = []
                for link in links:
                    text = link.get("text", "")
                    href = link.get("href", "")
                    lines.append(f"- [{text}]({href})")
                content = "\n".join(lines) if lines else "No links found."
                if len(content) > 10000:
                    content = content[:10000] + "\n\n[Content truncated]"
                return ToolResult(
                    tool_name="browser_extract",
                    content=content,
                    success=True,
                    metadata={
                        "selector": selector,
                        "extract_type": extract_type,
                        "num_links": len(links),
                    },
                )

            else:  # tables
                tables_text = page.eval_on_selector_all(
                    f"{selector} table",
                    """elements => elements.map(el => el.innerText)""",
                )
                if tables_text:
                    content = "\n\n---\n\n".join(tables_text)
                else:
                    content = "No tables found."
                if len(content) > 10000:
                    content = content[:10000] + "\n\n[Content truncated]"
                return ToolResult(
                    tool_name="browser_extract",
                    content=content,
                    success=True,
                    metadata={
                        "selector": selector,
                        "extract_type": extract_type,
                        "num_tables": len(tables_text),
                    },
                )

        except ImportError:
            return ToolResult(
                tool_name="browser_extract",
                content=(
                    "playwright not installed. Install with: uv sync --extra browser"
                ),
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_extract",
                content=f"Extract error: {exc}",
                success=False,
            )


# ---------------------------------------------------------------------------
# Tool 6: BrowserGetElementsTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_get_elements")
class BrowserGetElementsTool(BaseTool):
    """Get all interactive elements on the current page."""

    tool_id = "browser_get_elements"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_get_elements",
            description=(
                "Get a list of all interactive elements (inputs, buttons, links)"
                " on the current browser page. Returns their tag, text, and selector."
                " Use this to discover how to interact with a new website."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": (
                            "Optional CSS selector to limit the search area. Default: 'body'."
                        ),
                    },
                },
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        root_selector = params.get("selector", "body")

        try:
            page = _session.page
            # JavaScript to find interactive elements and build a friendly list
            script = """
            (selector) => {
                const root = document.querySelector(selector) || document.body;
                const elements = root.querySelectorAll('button, input, select, textarea, a, [role="button"], [role="link"]');
                return Array.from(elements).map(el => {
                    const rect = el.getBoundingClientRect();
                    if (rect.width === 0 || rect.height === 0 || getComputedStyle(el).display === 'none') return null;
                    
                    let best_selector = el.id ? `#${el.id}` : '';
                    if (!best_selector && el.name) best_selector = `${el.tagName.toLowerCase()}[name="${el.name}"]`;
                    if (!best_selector && el.type === 'submit') best_selector = 'button[type="submit"]';
                    
                    return {
                        tag: el.tagName.toLowerCase(),
                        text: el.innerText.trim() || el.placeholder || el.value || el.ariaLabel || '',
                        type: el.type || '',
                        id: el.id || '',
                        name: el.name || '',
                        selector: best_selector || el.tagName.toLowerCase()
                    };
                }).filter(x => x !== null).slice(0, 50); // Limit to top 50
            }
            """
            results = page.evaluate(script, root_selector)
            
            # Format results as a readable string
            lines = [f"Found {len(results)} interactive elements:"]
            for i, res in enumerate(results):
                lines.append(f"{i+1}. <{res['tag']}> \"{res['text']}\" (Selector: `{res['selector']}`)")
            
            content = "\n".join(lines)
            
            # Auto-capture screenshot for visual feedback
            meta = _capture_metadata(page)
            
            return ToolResult(
                tool_name="browser_get_elements",
                content=content,
                success=True,
                metadata={**meta, "elements": results},
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_get_elements",
                content=f"Inspection error: {exc}",
                success=False,
            )


__all__ = [
    "BrowserNavigateTool",
    "BrowserClickTool",
    "BrowserTypeTool",
    "BrowserScreenshotTool",
    "BrowserExtractTool",
    "BrowserGetElementsTool",
]
