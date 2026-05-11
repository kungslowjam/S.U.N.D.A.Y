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
        # Dedup state: prevent the agent from calling the same tool and getting identical results
        self._last_extract_hash: str = ""
        self._last_elements_hash: str = ""
        self._extract_repeat_count: int = 0
        self._elements_repeat_count: int = 0

    @property
    def page(self):
        self._ensure_browser()
        return self._page

    def close(self) -> None:
        """Forcefully close the current session."""
        try:
            if self._page:
                self._page.close()
            if self._browser:
                self._browser.close()
            if self._playwright:
                self._playwright.stop()
        except Exception:
            pass
        self._page = None
        self._browser = None
        self._playwright = None

    def _ensure_browser(self) -> None:
        if self._page is not None:
            try:
                # Heartbeat check to see if the session is still alive
                self._page.evaluate("1")
                return
            except Exception:
                # Page or browser has been closed/died, reset everything
                self.close()

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
        
        # Spoof navigator.webdriver and other bot detection properties, plus Auto Popup Killer
        self._page.add_init_script("""
            Object.defineProperty(navigator, 'webdriver', {get: () => undefined});
            window.chrome = { runtime: {} };
            Object.defineProperty(navigator, 'languages', {get: () => ['th-TH', 'th', 'en-US', 'en']});
            Object.defineProperty(navigator, 'plugins', {get: () => [1, 2, 3, 4, 5]});
            
            // ═══════════════════════════════════════════════════════════════
            // JARVIS - Advanced Popup & Overlay Shield (V2 - Aggressive)
            // ═══════════════════════════════════════════════════════════════
            setInterval(() => {
                const closeSelectors = [
                    '[aria-label="Close"]', '[aria-label="close"]', '[aria-label="ปิด"]',
                    '.close-button', '.modal-close', '.popup-close', '.btn-close', 
                    '.fancybox-close', '.insider-opt-in-notification-close-button',
                    'img[src*="close"]', 'i.fa-times', 'i.fa-close', 'svg[data-icon="xmark"]',
                    'div[id*="close"]', 'div[class*="close"]', '.ab-close-button',
                    // Amazon & Travel Sites Specifics
                    '[data-action-type="DISMISS"]', '.a-button-close', '#nav-main .nav-sprite',
                    'input[data-action-type="SELECT_LOCATION"]',
                    '.bui-modal__close', '[aria-label="Dismiss sign-in info."]',
                    '.modal-mask-close', '.sgn-x', '.shopee-popup__close-btn'
                ];
                
                // 1. Target Modals & Dialogs directly
                document.querySelectorAll('[role="dialog"], [aria-modal="true"], .modal, .popup-container').forEach(el => {
                    if (!el.hasAttribute('data-sunday-handled-modal')) {
                        console.log('[Jarvis] Detected Modal, searching for exit...');
                        // Look for any small button or X-like element inside
                        const possibleButtons = el.querySelectorAll('button, [role="button"], i, svg');
                        possibleButtons.forEach(btn => {
                            const rect = btn.getBoundingClientRect();
                            if (rect.width < 50 && rect.height < 50) { // Likely a close 'X'
                                btn.click();
                            }
                        });
                        el.setAttribute('data-sunday-handled-modal', 'true');
                    }
                });

                // 2. Click Close buttons with force
                document.querySelectorAll(closeSelectors.join(',')).forEach(el => {
                    const style = window.getComputedStyle(el);
                    if(style.display !== 'none' && style.visibility !== 'hidden' && el.getBoundingClientRect().width > 0) {
                        try { 
                            if (!el.hasAttribute('data-sunday-handled')) {
                                el.click(); 
                                el.setAttribute('data-sunday-handled', 'true');
                            }
                        } catch(e){}
                    }
                });

                // 3. GHOST MODE: Hide persistent blockers that cover too much screen
                document.querySelectorAll('div').forEach(el => {
                    const rect = el.getBoundingClientRect();
                    const style = window.getComputedStyle(el);
                    if (style.position === 'fixed' && 
                        rect.width > window.innerWidth * 0.5 && 
                        rect.height > window.innerHeight * 0.5 &&
                        style.zIndex > 1000) {
                        // If it's been here too long, ghost it
                        const startTime = parseInt(el.getAttribute('data-sunday-start') || Date.now());
                        el.setAttribute('data-sunday-start', startTime);
                        if (Date.now() - startTime > 5000) {
                            console.log('[Jarvis] Ghosting persistent blocker:', el);
                            el.style.display = 'none';
                            el.style.pointerEvents = 'none';
                        }
                    }
                });
            }, 1000); // Check every 1 second instead of 2

            // 3. JARVIS Visual Indicator (Blue Glow)
            // Note: Keyframes moved to Shadow DOM for encapsulation
        """)
        
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
                    0% { box-shadow: inset 0 0 40px 10px rgba(0, 242, 254, 0.4), 0 0 0 0px rgba(0, 242, 254, 0.6); }
                    50% { box-shadow: inset 0 0 80px 30px rgba(0, 242, 254, 0.7), 0 0 20px 5px rgba(0, 242, 254, 0.4); }
                    100% { box-shadow: inset 0 0 40px 10px rgba(0, 242, 254, 0.4), 0 0 0 0px rgba(0, 242, 254, 0.6); }
                }
                @keyframes sunday-glow-pulse {
                    0% { box-shadow: 0 0 10px #00f2fe, inset 0 0 5px #00f2fe; border-color: #00f2fe; }
                    50% { box-shadow: 0 0 25px #4facfe, inset 0 0 15px #4facfe; border-color: #4facfe; }
                    100% { box-shadow: 0 0 10px #00f2fe, inset 0 0 5px #00f2fe; border-color: #00f2fe; }
                }
                @keyframes sunday-ripple {
                    0% { transform: scale(0); opacity: 1; border-width: 4px; }
                    100% { transform: scale(4); opacity: 0; border-width: 1px; }
                }
                .sunday-target-highlight {
                    animation: sunday-glow-pulse 1s infinite ease-in-out !important;
                    outline: 2px solid #00f2fe !important;
                    border-radius: 6px !important;
                    z-index: 2147483645 !important;
                    transition: all 0.3s ease !important;
                }
                .sunday-click-ripple {
                    position: fixed;
                    width: 30px;
                    height: 30px;
                    border: 2px solid #00f2fe;
                    border-radius: 50%;
                    pointer-events: none;
                    z-index: 2147483647;
                    animation: sunday-ripple 0.6s ease-out;
                    transform: translate(-50%, -50%);
                }
                #sunday-cursor-status {
                    position: absolute;
                    bottom: 25px;
                    left: 20px;
                    background: rgba(10, 10, 20, 0.9);
                    color: #00f2fe;
                    padding: 4px 12px;
                    border-radius: 20px;
                    font-size: 11px;
                    font-weight: bold;
                    font-family: 'Inter', sans-serif;
                    white-space: nowrap;
                    border: 1px solid #00f2fe;
                    box-shadow: 0 0 10px rgba(0, 242, 254, 0.4);
                    pointer-events: none;
                    backdrop-filter: blur(4px);
                    opacity: 0;
                    transition: opacity 0.3s ease;
                }
            `;
            shadow.appendChild(style);
            shadow.appendChild(border);
            
            // 2. Persistent Cursor (The Red Dot)
            if (!window.__playwright_cursor) {
                window.__playwright_cursor = document.createElement('div');
                Object.assign(window.__playwright_cursor.style, {
                    position: 'fixed',
                    width: '16px',
                    height: '16px',
                    background: 'radial-gradient(circle, #00f2fe 0%, #4facfe 100%)',
                    border: '2px solid #fff',
                    borderRadius: '50%',
                    pointerEvents: 'none',
                    zIndex: '2147483647',
                    boxShadow: '0 0 15px #00f2fe, 0 0 30px rgba(0, 242, 254, 0.4)',
                    display: 'none'
                });
                
                const status = document.createElement('div');
                status.id = 'sunday-cursor-status';
                window.__playwright_cursor.appendChild(status);
                
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
            window.__show_scanner(); 
            const interactiveRoles = ['button', 'link', 'checkbox', 'menuitem', 'option', 'tab', 'textbox', 'combobox'];
            const tree = [];
            let idCounter = 1;
            
            const processNode = (node) => {
                if (!node || !node.querySelectorAll) return;
                const elements = node.querySelectorAll('button, a, input, select, textarea, [role], [onclick], span, div');
                elements.forEach(el => {
                    // Avoid duplicates if already processed in this turn
                    if (el.hasAttribute('data-sunday-id')) return;

                    const rect = el.getBoundingClientRect();
                    const style = window.getComputedStyle(el);
                    
                    if (rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden') {
                        const role = el.getAttribute('role') || el.tagName.toLowerCase();
                        const text = (el.innerText || el.value || el.placeholder || el.getAttribute('aria-label') || '').trim();
                        
                        const isClickable = style.cursor === 'pointer' || el.onclick || el.hasAttribute('onclick') || interactiveRoles.includes(role);
                        
                        // Logic to include meaningful elements
                        if (text.length > 0 || isClickable) {
                            // Filter out giant layout containers unless they are interactive
                            if (rect.width < 1000 || isClickable || ['a', 'button', 'input'].includes(role)) {
                                const id = idCounter++;
                                el.setAttribute('data-sunday-id', id);
                                tree.push({
                                    id,
                                    role,
                                    text: text.substring(0, 100).replace(/\n/g, ' '),
                                    x: Math.round(rect.left + rect.width / 2),
                                    y: Math.round(rect.top + rect.height / 2)
                                });
                            }
                        }
                    }
                    // Deep crawl into shadow roots
                    if (el.shadowRoot) processNode(el.shadowRoot);
                });
            };
            
            processNode(document);
            return tree;
        };

        window.__move_cursor = (x, y, statusText = '') => {
            if (!window.__playwright_cursor) return;
            window.__playwright_cursor.style.display = 'block';
            
            const statusEl = window.__playwright_cursor.querySelector('#sunday-cursor-status');
            if (statusEl) {
                if (statusText) {
                    statusEl.innerText = statusText;
                    statusEl.style.opacity = '1';
                } else {
                    statusEl.style.opacity = '0';
                }
            }

            const currentLeft = parseFloat(window.__playwright_cursor.style.left) || x;
            const currentTop = parseFloat(window.__playwright_cursor.style.top) || y;
            
            const dx = x - currentLeft;
            const dy = y - currentTop;
            const dist = Math.sqrt(dx*dx + dy*dy);
            
            function createTrail(tx, ty) {
                const trail = document.createElement('div');
                Object.assign(trail.style, {
                    position: 'fixed', left: tx + 'px', top: ty + 'px',
                    width: '4px', height: '4px', backgroundColor: 'rgba(0, 242, 254, 0.4)',
                    borderRadius: '50%', transform: 'translate(-50%, -50%)',
                    pointerEvents: 'none', zIndex: '2147483646', transition: 'opacity 0.4s ease-out'
                });
                document.body.appendChild(trail);
                setTimeout(() => { trail.style.opacity = '0'; }, 50);
                setTimeout(() => trail.remove(), 500);
            }
            
            function createRipple(rx, ry) {
                const ripple = document.createElement('div');
                ripple.className = 'sunday-click-ripple';
                ripple.style.left = rx + 'px';
                ripple.style.top = ry + 'px';
                document.body.appendChild(ripple);
                setTimeout(() => ripple.remove(), 600);
            }

            if (dist < 10) {
                window.__playwright_cursor.style.left = x + 'px';
                window.__playwright_cursor.style.top = y + 'px';
                if (statusText.toLowerCase().includes('click')) createRipple(x, y);
                return;
            }

            const duration = Math.min(800, Math.max(300, dist * 1.5));
            const startTime = performance.now();
            let lastTrailTime = startTime;

            const animate = (time) => {
                let progress = (time - startTime) / duration;
                if (progress > 1) progress = 1;
                const easeOut = 1 - Math.pow(1 - progress, 4);
                const curX = currentLeft + dx * easeOut;
                const curY = currentTop + dy * easeOut;
                
                window.__playwright_cursor.style.left = curX + 'px';
                window.__playwright_cursor.style.top = curY + 'px';
                
                if (time - lastTrailTime > 30) {
                    createTrail(curX, curY);
                    lastTrailTime = time;
                }
                
                if (progress < 1) requestAnimationFrame(animate);
                else if (statusText.toLowerCase().includes('click')) createRipple(x, y);
            };
            requestAnimationFrame(animate);
        };

        window.__highlight_target = (selector) => {
            document.querySelectorAll('.sunday-target-highlight').forEach(el => el.classList.remove('sunday-target-highlight'));
            const target = typeof selector === 'string' ? document.querySelector(selector) : selector;
            if (target) target.classList.add('sunday-target-highlight');
        };

            if (dist < 5) {
                window.__playwright_cursor.style.left = x + 'px';
                window.__playwright_cursor.style.top = y + 'px';
                createRipple(x, y);
                return;
            }

            const duration = 400; // ms
            const startTime = performance.now();
            let lastTrailTime = startTime;

            const animate = (time) => {
                let progress = (time - startTime) / duration;
                if (progress > 1) progress = 1;
                
                // Ease out cubic
                const easeOut = 1 - Math.pow(1 - progress, 3);
                
                const curX = currentLeft + dx * easeOut;
                const curY = currentTop + dy * easeOut;
                
                window.__playwright_cursor.style.left = curX + 'px';
                window.__playwright_cursor.style.top = curY + 'px';
                
                // Create trail at a steady rate
                if (time - lastTrailTime > 20) {
                    createTrail(curX, curY);
                    lastTrailTime = time;
                }

                if (progress < 1) {
                    requestAnimationFrame(animate);
                } else {
                    createRipple(x, y);
                }
            };
            
            requestAnimationFrame(animate);
        };
        """
        try:
            page.add_init_script(script)
            page.evaluate(script) # Immediate injection
        except Exception:
            pass

    def _visual_move(self, page, x: float, y: float, status: str = "", selector: str = "") -> None:
        """Move the visual cursor to coordinates or selector with status."""
        try:
            if selector:
                page.evaluate(f"window.__highlight_target('{selector}')")
            page.evaluate(f"window.__move_cursor({x}, {y}, '{status}')")
            page.wait_for_timeout(500) # Wait for animation to finish
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
        screenshot_path = os.path.join(root, "last_screenshot.jpg")
        with open(screenshot_path, "wb") as f:
            f.write(screenshot_bytes)
        
        warning = _check_page_validity(page)
        msg = f"Screenshot saved to {screenshot_path}"
        if warning:
            msg = f"{warning}\n{msg}"
            
        return {"screenshot_base64": b64_data, "message": msg}
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
        wait_for = params.get("wait_for", "load")
        
        # 🛡️ URL Sanitization: Handle cases where model sends JSON-like string
        if isinstance(url, str) and url.startswith('{') and 'url' in url:
            import json
            try:
                data = json.loads(url)
                url = data.get('url', url)
            except:
                pass
        
        url = str(url).strip().strip('"').strip("'")
        
        if not url:
            return ToolResult(
                tool_name="browser_navigate",
                content="No URL provided.",
                success=False,
            )

        if not url.startswith("http") and "." in url:
            url = f"https://{url}"

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
            # Reset dedup state — new page means new content
            _session._last_extract_hash = ""
            _session._last_elements_hash = ""
            _session._extract_repeat_count = 0
            _session._elements_repeat_count = 0
            page.goto(url, wait_until=wait_for)
            
            # Detect Cloudflare / Bot detection / Security Verification
            title = page.title()
            security_keywords = ["Just a moment...", "Checking your browser", "security verification", "Verify you are human", "Cloudflare", "Attention Required!"]
            
            if any(k.lower() in title.lower() for k in security_keywords):
                # 1. Attempt Auto-Click for common challenges using Real Mouse movements
                try:
                    # Find the challenge element coordinates (even inside iframes)
                    box = page.evaluate("""() => {
                        const sel = ['input[type="checkbox"]', '#challenge-stage', '.ctp-checkbox-label', '#cf-stage', '.cf-turnstile-wrapper', '.mark'];
                        for (const s of sel) {
                            const el = document.querySelector(s);
                            if (el && el.getBoundingClientRect().width > 0) {
                                const r = el.getBoundingClientRect();
                                return { x: r.left + r.width/2, y: r.top + r.height/2 };
                            }
                        }
                        // Check iframes for Turnstile
                        for (const f of document.querySelectorAll('iframe')) {
                            try {
                                const d = f.contentDocument || f.contentWindow.document;
                                for (const s of sel) {
                                    const el = d.querySelector(s);
                                    if (el && el.getBoundingClientRect().width > 0) {
                                        const fr = f.getBoundingClientRect();
                                        const er = el.getBoundingClientRect();
                                        return { x: fr.left + er.left + er.width/2, y: fr.top + er.top + er.height/2 };
                                    }
                                }
                            } catch(e){}
                        }
                        return null;
                    }""")
                    
                    if box:
                        # Perform human-like mouse movement and click
                        page.mouse.move(box['x'], box['y'], steps=15)
                        page.mouse.click(box['x'], box['y'], delay=120)
                except:
                    pass

                # 2. Wait for challenge to pass (max 20s)
                try:
                    page.wait_for_function(
                        f"() => !({ ' || '.join([f'document.title.toLowerCase().includes(\"{k.lower()}\")' for k in security_keywords]) })", 
                        timeout=20000
                    )
                    title = page.title()
                except Exception:
                    pass # Continue anyway, user can solve it manually
                    
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

@ToolRegistry.register("browser_reset")
class BrowserResetTool(BaseTool):
    """Forcefully restarts the browser session. Use if the browser hangs or gives errors."""
    
    spec = ToolSpec(
        name="browser_reset",
        description="Forcefully restarts the shared browser session.",
        parameters={"type": "object", "properties": {}},
    )

    def execute(self, **params: Any) -> ToolResult:
        _session.close()
        _session._ensure_browser()
        return ToolResult(
            tool_name="browser_reset",
            content="Browser session has been forcefully restarted.",
            success=True,
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
            
            # 1. Target by Numeric ID (@id)
            if selector.startswith("@"):
                target_id = selector[1:]
                box = page.evaluate(f"""() => {{
                    const el = document.querySelector('[data-sunday-id="{target_id}"]');
                    if (el) {{
                        const r = el.getBoundingClientRect();
                        return {{ x: r.left + r.width/2, y: r.top + r.height/2 }};
                    }}
                    return null;
                }}""")
                if box:
                    _session._visual_move(page, box['x'], box['y'], status="Clicking...", selector=f"[data-sunday-id='{target_id}']")
                    page.mouse.click(box['x'], box['y'])
                    return ToolResult(tool_name="browser_click", content=f"Clicked element @{target_id}", success=True)
                return ToolResult(tool_name="browser_click", content=f"Element @{target_id} not found", success=False)

            # 2. Target by Text (with fuzzy match and Shadow DOM)
            if by_text or "contains" in selector:
                # Clean selector if it was 'button:contains("...")'
                clean_text = selector
                if "contains('" in selector:
                    clean_text = selector.split("'")[1]
                elif 'contains("' in selector:
                    clean_text = selector.split('"')[1]
                
                # Safety guard: Don't click on ambiguous 1-char strings (prevents 'a', 'i' hallucinations)
                if len(clean_text) < 2:
                    return ToolResult(tool_name="browser_click", content=f"Text target '{clean_text}' is too short and ambiguous. Try a more specific word.", success=False)
                
                box = page.evaluate(f"""() => {{
                    const target = "{clean_text.replace('"', '\\"').lower()}";
                    const crawl = (root) => {{
                        const nodes = root.querySelectorAll('button, a, span, div, input, [role="button"]');
                        for (const node of nodes) {{
                            const txt = (node.innerText || node.value || node.placeholder || "").toLowerCase();
                            if (txt.includes(target)) {{
                                const r = node.getBoundingClientRect();
                                if (r.width > 0 && r.height > 0) return {{ x: r.left + r.width/2, y: r.top + r.height/2 }};
                            }}
                            if (node.shadowRoot) {{
                                const res = crawl(node.shadowRoot);
                                if (res) return res;
                            }}
                        }}
                        return null;
                    }};
                    return crawl(document);
                }}""")
                
                if box:
                    _session._visual_move(page, box['x'], box['y'], status="Clicking...")
                    page.mouse.click(box['x'], box['y'])
                    return ToolResult(tool_name="browser_click", content=f"Clicked text '{clean_text}' via coordinates", success=True)

            # 3. Target by Standard CSS Selector
            try:
                page.wait_for_selector(selector, timeout=3000, state="visible")
                el = page.locator(selector).first
                box = el.bounding_box()
                if box:
                    _session._visual_move(page, box['x'] + box['width']/2, box['y'] + box['height']/2, status="Clicking...", selector=selector)
                page.click(selector, delay=50)
                return ToolResult(tool_name="browser_click", content=f"Clicked selector '{selector}'", success=True)
            except Exception as e:
                return ToolResult(tool_name="browser_click", content=f"Could not click '{selector}': {e}", success=False)

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

            if selector.startswith("@"):
                # Numeric ID targeting
                target_id = selector[1:]
                loc = page.locator(f'[data-sunday-id="{target_id}"]').first
            else:
                # --- FIX: Escape special CSS chars (e.g. ':' in React IDs like #:R55amr5:)
                import re as _re
                safe_selector = selector
                # If it's an ID selector with special chars, use attribute selector instead
                if selector.startswith('#') and _re.search(r'[:\[\]()]', selector[1:]):
                    raw_id = selector[1:]
                    safe_selector = f'[id="{raw_id}"]'
                loc = page.locator(safe_selector).first

            # Ensure element is ready
            try:
                loc.wait_for(state="visible", timeout=3000)
            except:
                pass

            import random
            # Move to element visually
            try:
                box = loc.bounding_box()
                if box:
                    _session._visual_move(page, box['x'] + box['width']/2, box['y'] + box['height']/2, status="Typing...", selector=selector)
            except:
                pass
            
            if clear:
                try: loc.fill("") # Clear first
                except: pass
            
            # Type with human-like delay
            try:
                loc.click() # Focus first
                for char in text:
                    loc.type(char, delay=random.randint(30, 80))
            except Exception as e:
                # Fallback to direct fill if slow typing fails
                loc.fill(text)
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
                
                # Dedup guard
                import hashlib
                content_hash = hashlib.md5(content[:500].encode()).hexdigest()
                if content_hash == _session._last_extract_hash:
                    _session._extract_repeat_count += 1
                    if _session._extract_repeat_count >= 2:
                        return ToolResult(
                            tool_name="browser_extract",
                            content="STOP: Page content is identical to the last extraction. You already have this data. Summarize and present your findings now.",
                            success=True,
                            metadata={"deduplicated": True},
                        )
                else:
                    _session._last_extract_hash = content_hash
                    _session._extract_repeat_count = 0
                
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
                const elements = root.querySelectorAll('button, input, select, textarea, a, [role="button"], [role="link"], [role="searchbox"]');
                
                // Generic noise keywords (Strict)
                const noiseKeywords = [
                    'language', 'currency', 'sign in', 'register', 'log in', 'create account',
                    'help', 'support', 'privacy', 'terms', 'cookies', 'about us', 'contact us',
                    'skip to', 'navigation', 'footer', 'customer support', 'list your property',
                    'aud', 'usd', 'eur', 'gbp'
                ];
                
                let found = Array.from(elements).map(el => {
                    const rect = el.getBoundingClientRect();
                    // Basic visibility check
                    if (rect.width === 0 || rect.height === 0 || getComputedStyle(el).display === 'none' || getComputedStyle(el).visibility === 'hidden') return null;
                    
                    const text = (el.innerText.trim() || el.placeholder || el.value || el.ariaLabel || el.title || '').toLowerCase();
                    const tagName = el.tagName.toLowerCase();
                    const role = el.getAttribute('role') || '';

                    // Filter out non-functional noise
                    if (noiseKeywords.some(n => text.includes(n)) && text.length < 30) return null;

                    let best_selector = el.id ? `#${el.id}` : '';
                    if (!best_selector && el.name) best_selector = `${tagName}[name="${el.name}"]`;
                    if (!best_selector && el.type === 'submit') best_selector = `${tagName}[type="submit"]`;
                    
                    // Priority Scoring System (General)
                    let score = 0;
                    if (tagName === 'input' || tagName === 'textarea' || role === 'searchbox') score += 10; // Inputs are primary
                    if (el.type === 'submit' || text.includes('search') || text.includes('find') || text.includes('go')) score += 8; // Submit/Search buttons
                    if (rect.top < window.innerHeight / 2) score += 2; // Favor elements in the top half (usually search bars)
                    if (el.offsetParent === null) score -= 10; // Hidden or detached

                    return {
                        tag: tagName,
                        text: el.innerText.trim() || el.placeholder || el.value || el.ariaLabel || '',
                        type: el.type || '',
                        id: el.id || '',
                        name: el.name || '',
                        selector: best_selector || tagName,
                        score: score
                    };
                }).filter(x => x !== null);

                // Sort by priority score and return top 30 most relevant interactive elements
                return found.sort((a, b) => b.score - a.score).slice(0, 30);
            }
            """
            results = page.evaluate(script, root_selector)
            
            # Dedup guard: detect if elements are identical to last call
            import hashlib
            elements_sig = str([(r.get('tag'), r.get('text','')[:30], r.get('selector')) for r in results[:10]])
            elements_hash = hashlib.md5(elements_sig.encode()).hexdigest()
            if elements_hash == _session._last_elements_hash:
                _session._elements_repeat_count += 1
                if _session._elements_repeat_count >= 2:
                    return ToolResult(
                        tool_name="browser_get_elements",
                        content="STOP: Elements are identical to the last call. The page has not changed. Try a different action (click, navigate, or summarize your findings).",
                        success=True,
                        metadata={"deduplicated": True},
                    )
            else:
                _session._last_elements_hash = elements_hash
                _session._elements_repeat_count = 0
            
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
