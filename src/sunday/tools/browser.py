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
        self._browser = self._playwright.chromium.launch(
            headless=False,
            args=[
                "--disable-blink-features=AutomationControlled",
                "--no-sandbox",
                "--disable-infobars"
            ]
        )
        self._page = self._browser.new_page(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            viewport={"width": 1280, "height": 720},
            locale="th-TH",
            timezone_id="Asia/Bangkok"
        )
        self._setup_cursor(self._page)

    def _setup_cursor(self, page) -> None:
        """Inject a visual cursor script into the page."""
        script = """
        const injectStyles = () => {
            if (document.getElementById('__sunday_visuals_root')) return;
            
            const root = document.createElement('div');
            root.id = '__sunday_visuals_root';
            const shadow = root.attachShadow({mode: 'open'});
            
            // 1. Pulsing Blue Glow Border
            const border = document.createElement('div');
            Object.assign(border.style, {
                position: 'fixed',
                top: '0',
                left: '0',
                width: '100vw',
                height: '100vh',
                boxSizing: 'border-box',
                pointerEvents: 'none',
                zIndex: '2147483647',
                border: '8px solid rgba(0, 123, 255, 0.6)',
                boxShadow: 'inset 0 0 80px 30px rgba(0, 123, 255, 0.5), 0 0 40px rgba(0, 123, 255, 0.3)',
                animation: 'sunday-pulse 2s infinite ease-in-out',
                display: 'block'
            });
            const style = document.createElement('style');
            style.textContent = `
                @keyframes sunday-pulse {
                    0% { opacity: 0.7; box-shadow: inset 0 0 60px 20px rgba(0, 123, 255, 0.4); }
                    50% { opacity: 1; box-shadow: inset 0 0 100px 40px rgba(0, 123, 255, 0.6); }
                    100% { opacity: 0.7; box-shadow: inset 0 0 60px 20px rgba(0, 123, 255, 0.4); }
                }
            `;
            shadow.appendChild(style);
            shadow.appendChild(border);
            
            // 2. Persistent Cursor (The Red Dot)
            if (!window.__playwright_cursor) {
                window.__playwright_cursor = document.createElement('div');
                Object.assign(window.__playwright_cursor.style, {
                    position: 'fixed',
                    width: '20px',
                    height: '20px',
                    backgroundColor: 'rgba(255, 0, 0, 0.8)',
                    border: '3px solid white',
                    borderRadius: '50%',
                    pointerEvents: 'none',
                    zIndex: '2147483647',
                    transition: 'all 0.4s cubic-bezier(0.165, 0.84, 0.44, 1)',
                    transform: 'translate(-50%, -50%)',
                    display: 'none',
                    boxShadow: '0 0 15px rgba(0,0,0,0.6)'
                });
                shadow.appendChild(window.__playwright_cursor);
            }
            
            (document.body || document.documentElement).appendChild(root);
        };

        window.__show_scanner = () => {
            const scanner = document.createElement('div');
            Object.assign(scanner.style, {
                position: 'fixed',
                top: '-10px',
                left: '0',
                width: '100vw',
                height: '8px',
                backgroundColor: 'rgba(0, 123, 255, 0.8)',
                boxShadow: '0 0 20px rgba(0, 123, 255, 1)',
                zIndex: '2147483647',
                pointerEvents: 'none',
                transition: 'top 1.2s ease-in-out'
            });
            document.documentElement.appendChild(scanner);
            setTimeout(() => { scanner.style.top = '100vh'; }, 20);
            setTimeout(() => scanner.remove(), 1300);
        };

        setInterval(injectStyles, 500);
        injectStyles();

        window.__get_ax_tree = () => {
            window.__show_scanner(); // Show laser scan when reading elements
            const interactiveRoles = ['button', 'link', 'checkbox', 'menuitem', 'option', 'tab', 'textbox'];
            const elements = Array.from(document.querySelectorAll('button, a, input, select, textarea, [role]'));
            const tree = [];
            let idCounter = 1;
            
            elements.forEach(el => {
                const rect = el.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0 && window.getComputedStyle(el).display !== 'none') {
                    const role = el.getAttribute('role') || el.tagName.toLowerCase();
                    const text = el.innerText || el.value || el.placeholder || el.getAttribute('aria-label') || '';
                    if (text.trim() || interactiveRoles.includes(role)) {
                        const id = idCounter++;
                        el.setAttribute('data-sunday-id', id);
                        tree.push({
                            id,
                            role,
                            text: text.trim().substring(0, 100),
                            x: Math.round(rect.left + rect.width / 2),
                            y: Math.round(rect.top + rect.height / 2)
                        });
                    }
                }
            });
            return tree;
        };

        window.__move_cursor = (x, y) => {
            if (!window.__playwright_cursor) return;
            window.__playwright_cursor.style.display = 'block';
            window.__playwright_cursor.style.left = x + 'px';
            window.__playwright_cursor.style.top = y + 'px';
            
            const trail = document.createElement('div');
            Object.assign(trail.style, {
                position: 'fixed',
                left: x + 'px',
                top: y + 'px',
                width: '8px',
                height: '8px',
                backgroundColor: 'rgba(255, 0, 0, 0.5)',
                borderRadius: '50%',
                transform: 'translate(-50%, -50%)',
                pointerEvents: 'none',
                zIndex: '2147483646',
                transition: 'opacity 1.5s ease-out'
            });
            document.body.appendChild(trail);
            setTimeout(() => { trail.style.opacity = '0'; }, 200);
            setTimeout(() => trail.remove(), 1600);

            const ripple = document.createElement('div');
            Object.assign(ripple.style, {
                position: 'fixed',
                left: x + 'px',
                top: y + 'px',
                width: '1px',
                height: '1px',
                border: '4px solid red',
                borderRadius: '50%',
                transform: 'translate(-50%, -50%)',
                pointerEvents: 'none',
                zIndex: '2147483646',
                transition: 'all 0.6s ease-out'
            });
            document.body.appendChild(ripple);
            setTimeout(() => {
                ripple.style.width = '60px';
                ripple.style.height = '60px';
                ripple.style.opacity = '0';
            }, 10);
            setTimeout(() => ripple.remove(), 700);
        };
        """
        try:
            page.add_init_script(script)
            page.evaluate(script) # Immediate injection
        except Exception:
            pass

    def _visual_move(self, page, selector: str) -> None:
        """Move the visual cursor to an element before action."""
        try:
            # Ensure script is there (in case of navigation)
            page.evaluate("window.__move_cursor && window.__move_cursor(0,0)")
            el = page.wait_for_selector(selector, timeout=2000)
            if el:
                box = el.bounding_box()
                if box:
                    x = box['x'] + box['width'] / 2
                    y = box['y'] + box['height'] / 2
                    page.evaluate(f"window.__move_cursor({x}, {y})")
                    page.wait_for_timeout(400) # Wait for animation
        except Exception:
            pass

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
    """Capture optimized screenshot for fast visual feedback."""
    try:
        import base64
        import os
        # Capture fast JPEG screenshot with lower quality
        screenshot_bytes = page.screenshot(type="jpeg", quality=50)
        b64_data = base64.b64encode(screenshot_bytes).decode("utf-8")
        
        # Save to last_screenshot.jpg (renamed for clarity)
        root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
        with open(os.path.join(root, "last_screenshot.jpg"), "wb") as f:
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
                page.wait_for_selector(selector, timeout=2000)
            except Exception:
                pass # Continue anyway, maybe it's there
                
            if selector.startswith("@"):
                # Numeric ID targeting
                target_id = selector[1:]
                page.evaluate(f"""
                    const el = document.querySelector('[data-sunday-id="{target_id}"]');
                    if (el) el.click();
                """)
                # Also move cursor visually
                res = page.evaluate(f"""
                    const el = document.querySelector('[data-sunday-id="{target_id}"]');
                    if (el) {{
                        const r = el.getBoundingClientRect();
                        window.__move_cursor(r.left + r.width/2, r.top + r.height/2);
                    }}
                """)
                page.wait_for_timeout(500)
            elif by_text:
                _session._visual_move(page, f"text={selector}")
                page.get_by_text(selector).first.click()
            else:
                _session._visual_move(page, selector)
                page.locator(selector).first.click()
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
                page.wait_for_selector(selector, timeout=2000)
            except Exception:
                pass

            if selector.startswith("@"):
                # Numeric ID targeting
                target_id = selector[1:]
                loc = page.locator(f'[data-sunday-id="{target_id}"]').first
                if clear:
                    loc.fill("")
                
                import random
                for char in text:
                    loc.type(char, delay=random.randint(40, 100))
            else:
                _session._visual_move(page, selector)
                loc = page.locator(selector).first
                if clear:
                    loc.fill("") # Clear first
                
                # Type with human-like delay
                import random
                for char in text:
                    loc.type(char, delay=random.randint(40, 100))
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
                if len(content) > 3000:
                    content = content[:3000] + "\n\n[Content truncated for speed]"
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


# ---------------------------------------------------------------------------
# Tool 7: BrowserDragTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_drag")
class BrowserDragTool(BaseTool):
    """Perform a drag operation across coordinates."""

    tool_id = "browser_drag"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_drag",
            description=(
                "Perform a drag gesture across a sequence of pixel coordinates."
                " The first waypoint is clicked, then the mouse is dragged through each"
                " subsequent point and released at the last."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "waypoints": {
                        "type": "array",
                        "description": "List of {x, y} coordinates to drag through.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "x": {"type": "number"},
                                "y": {"type": "number"}
                            },
                            "required": ["x", "y"]
                        },
                        "minItems": 2
                    }
                },
                "required": ["waypoints"],
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        waypoints = params.get("waypoints", [])
        try:
            page = _session.page
            if not waypoints:
                return ToolResult(tool_name="browser_drag", content="No waypoints provided.", success=False)

            # Move to start
            start = waypoints[0]
            page.mouse.move(start['x'], start['y'])
            page.evaluate(f"window.__move_cursor && window.__move_cursor({start['x']}, {start['y']})")
            page.mouse.down()
            
            # Drag through points
            for pt in waypoints[1:]:
                page.mouse.move(pt['x'], pt['y'], steps=5)
                page.evaluate(f"window.__move_cursor && window.__move_cursor({pt['x']}, {pt['y']})")
            
            page.mouse.up()
            meta = _capture_metadata(page)

            return ToolResult(
                tool_name="browser_drag",
                content=f"Performed drag through {len(waypoints)} points.",
                success=True,
                metadata=meta
            )
        except Exception as exc:
            return ToolResult(tool_name="browser_drag", content=f"Drag error: {exc}", success=False)


# ---------------------------------------------------------------------------
# Tool 8: BrowserScrollTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_scroll")
class BrowserScrollTool(BaseTool):
    """Scroll the page by pixels or percentage."""

    tool_id = "browser_scroll"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_scroll",
            description="Scroll the current page by a specific amount.",
            parameters={
                "type": "object",
                "properties": {
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down"],
                        "description": "Scroll direction."
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Amount to scroll in pixels. Default: 300."
                    }
                },
                "required": ["direction"]
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        direction = params.get("direction", "down")
        amount = params.get("amount", 300)
        scroll_val = amount if direction == "down" else -amount

        try:
            page = _session.page
            page.evaluate(f"window.scrollBy(0, {scroll_val})")
            page.wait_for_timeout(100) # Faster return
            meta = _capture_metadata(page)

            return ToolResult(
                tool_name="browser_scroll",
                content=f"Scrolled {direction} by {amount} pixels.",
                success=True,
                metadata=meta
            )
        except Exception as exc:
            return ToolResult(tool_name="browser_scroll", content=f"Scroll error: {exc}", success=False)


# ---------------------------------------------------------------------------
# Tool 9: BrowserGetAccessibilityTreeTool
# ---------------------------------------------------------------------------


@ToolRegistry.register("browser_get_accessibility_tree")
class BrowserGetAccessibilityTreeTool(BaseTool):
    """Extract a clean list of interactive elements with IDs."""

    tool_id = "browser_get_accessibility_tree"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_get_accessibility_tree",
            description=(
                "Get a simplified tree of interactive elements (buttons, links, inputs)."
                " Each element is assigned a numeric ID (e.g., 1, 2, 3) which you can use"
                " in click/type tools by prefixing with '@' (e.g., '@5')."
            ),
            parameters={
                "type": "object",
                "properties": {},
                "required": [],
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        try:
            page = _session.page
            # Force re-injection just in case
            _session._setup_cursor(page)
            tree = page.evaluate("window.__get_ax_tree()")
            
            output = "Interactive Elements Found:\n"
            for el in tree:
                output += f"[@{el['id']}] {el['role']}: \"{el['text']}\"\n"
            
            if not tree:
                output = "No interactive elements found."

            return ToolResult(
                tool_name="browser_get_accessibility_tree",
                content=output,
                success=True,
                metadata={"tree": tree}
            )
        except Exception as exc:
            return ToolResult(tool_name="browser_get_accessibility_tree", content=f"AXTree error: {exc}", success=False)


__all__ = [
    "BrowserNavigateTool",
    "BrowserClickTool",
    "BrowserTypeTool",
    "BrowserScreenshotTool",
    "BrowserExtractTool",
    "BrowserGetElementsTool",
    "BrowserDragTool",
    "BrowserScrollTool",
    "BrowserGetAccessibilityTreeTool",
]
