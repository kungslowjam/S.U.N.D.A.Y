"""HTTP request tool — make HTTP requests with SSRF protection."""

from __future__ import annotations

import json
import logging
import os
import time
import re
import html
from typing import Any

import httpx

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.security.ssrf import check_ssrf
from sunday.tools._stubs import BaseTool, ToolSpec

logger = logging.getLogger(__name__)

def clean_html_if_needed(text: str, content_type: str = "") -> str:
    """Detect if the content is HTML, and if so, strip style/script/head tags,
    decode HTML entities, and collapse white space to optimize token usage by 90%+."""
    is_html = "text/html" in content_type.lower()
    if not is_html:
        # Check signature if content-type is missing or vague
        truncated_prefix = text[:1000].lower()
        is_html = "<html" in truncated_prefix or "<!doctype html" in truncated_prefix

    if not is_html:
        return text

    logger.debug("HTML content detected. Filtering tags and metadata to optimize token usage...")
    original_len = len(text)

    # 1. Strip scripts, styles, and head tags (and their contents)
    text = re.sub(r'(?is)<script\b[^>]*>.*?</script>', '', text)
    text = re.sub(r'(?is)<style\b[^>]*>.*?</style>', '', text)
    text = re.sub(r'(?is)<head\b[^>]*>.*?</head>', '', text)
    text = re.sub(r'(?is)<!--.*?-->', '', text) # HTML comments

    # 2. Strip all other tags, replacing them with a space to prevent word merging
    text = re.sub(r'<[^>]+>', ' ', text)

    # 3. Decode HTML entities
    text = html.unescape(text)

    # 4. Clean up whitespaces
    lines = [line.strip() for line in text.splitlines()]
    # Remove empty lines and collapse multiple spaces
    cleaned_lines = []
    for line in lines:
        collapsed = re.sub(r'\s+', ' ', line).strip()
        if collapsed:
            cleaned_lines.append(collapsed)

    cleaned_text = '\n'.join(cleaned_lines)
    new_len = len(cleaned_text)
    reduction = ((original_len - new_len) / original_len) * 100 if original_len > 0 else 0
    
    logger.info("HTML cleaned: %d -> %d chars (%.1f%% reduction)", original_len, new_len, reduction)
    return cleaned_text + f"\n\n[💡 SUNDAY Token Optimization: HTML cleaned to raw text (Reduced by {reduction:.1f}%)]"

# Maximum response body size: 1 MB
_MAX_RESPONSE_BYTES = 1_048_576

_ALLOWED_METHODS = frozenset({"GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"})


@ToolRegistry.register("http_request")
class HttpRequestTool(BaseTool):
    """Make HTTP requests to external APIs with SSRF protection."""

    tool_id = "http_request"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="http_request",
            description=(
                "Make an HTTP request to a URL."
                " Supports GET, POST, PUT, DELETE, PATCH,"
                " and HEAD methods. Includes SSRF protection"
                " against private IPs and cloud metadata."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to send the request to.",
                    },
                    "method": {
                        "type": "string",
                        "description": (
                            "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD)."
                            " Defaults to GET."
                        ),
                    },
                    "headers": {
                        "type": "object",
                        "description": "Optional HTTP headers as key-value pairs.",
                    },
                    "body": {
                        "type": "string",
                        "description": "Optional request body (for POST, PUT, PATCH).",
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Request timeout in seconds. Defaults to 30.",
                    },
                },
                "required": ["url"],
            },
            category="network",
            required_capabilities=["network:fetch"],
        )

    def execute(self, **params: Any) -> ToolResult:
        url = params.get("url", "")
        if not url:
            return ToolResult(
                tool_name="http_request",
                content="No URL provided.",
                success=False,
            )

        method = params.get("method", "GET").upper()
        if method not in _ALLOWED_METHODS:
            return ToolResult(
                tool_name="http_request",
                content=(
                    f"Unsupported HTTP method: {method}."
                    f" Allowed: {', '.join(sorted(_ALLOWED_METHODS))}."
                ),
                success=False,
            )

        # SSRF protection check
        ssrf_error = check_ssrf(url)
        if ssrf_error:
            return ToolResult(
                tool_name="http_request",
                content=f"SSRF protection blocked request: {ssrf_error}",
                success=False,
            )

        headers = {
            k: os.path.expandvars(v) if isinstance(v, str) else v
            for k, v in (params.get("headers") or {}).items()
        }
        
        # Add default premium User-Agent to avoid generic crawler WAF blockages
        if not any(k.lower() == "user-agent" for k in headers):
            headers["User-Agent"] = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"

        body = params.get("body")
        timeout = params.get("timeout", 30)

        def add_scraper_tip_if_needed(text: str) -> str:
            text_lower = text.lower()
            if (
                "cloudflare" in text_lower
                or "403 forbidden" in text_lower
                or "access denied" in text_lower
                or "enable javascript" in text_lower
                or "security check" in text_lower
                or "block" in text_lower
            ):
                return text + (
                    "\n\n[⚠️ SUNDAY Scraper Tip]: It appears this website is blocking direct HTTP scraping (e.g. Cloudflare, WAF, or 403 Forbidden) or requires JavaScript to load dynamically. "
                    "Do NOT retry this HTTP request. Instead, please use 'web_search' to find the information, or use the browser automation tools "
                    "(like 'browser_navigate' and 'browser_extract') which emulate a real user in a browser environment!"
                )
            return text

        # Prefer Rust backend (now supports headers)
        try:
            from sunday._rust_bridge import get_rust_module

            _rust = get_rust_module()
            headers_json = json.dumps(headers) if headers else None
            content = _rust.HttpRequestTool().execute(url, method, body, headers_json)
            content = clean_html_if_needed(content)
            content = add_scraper_tip_if_needed(content)
            
            return ToolResult(
                tool_name="http_request",
                content=(
                    content[:_MAX_RESPONSE_BYTES]
                    if len(content) > _MAX_RESPONSE_BYTES
                    else content
                ),
                success=True,
                metadata={
                    "status_code": 200,
                    "truncated": len(content) > _MAX_RESPONSE_BYTES,
                },
            )
        except ImportError:
            pass  # Fall through to httpx below
        except Exception as exc:
            logger.debug("Rust HTTP request failed, fallback to httpx: %s", exc)

        # Python fallback
        try:
            t0 = time.time()
            response = httpx.request(
                method,
                url,
                headers=headers,
                content=body,
                timeout=float(timeout),
                follow_redirects=True,
            )
            elapsed_ms = (time.time() - t0) * 1000

            content_type = response.headers.get("content-type", "")
            response_headers = dict(response.headers)

            # Truncate response body if larger than 1 MB
            raw_body = response.text
            truncated = False
            if len(raw_body) > _MAX_RESPONSE_BYTES:
                raw_body = raw_body[:_MAX_RESPONSE_BYTES]
                truncated = True

            content = raw_body
            if truncated:
                content += "\n\n[Response truncated at 1 MB]"

            content = clean_html_if_needed(content, content_type)
            content = add_scraper_tip_if_needed(content)

            return ToolResult(
                tool_name="http_request",
                content=content,
                success=response.status_code < 400,
                metadata={
                    "status_code": response.status_code,
                    "headers": response_headers,
                    "content_type": content_type,
                    "elapsed_ms": round(elapsed_ms, 2),
                    "truncated": truncated,
                },
            )
        except httpx.TimeoutException as exc:
            return ToolResult(
                tool_name="http_request",
                content=f"Request timed out after {timeout}s: {exc}",
                success=False,
            )
        except httpx.RequestError as exc:
            return ToolResult(
                tool_name="http_request",
                content=f"Request error: {exc}",
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="http_request",
                content=f"Unexpected error: {exc}",
                success=False,
            )


__all__ = ["HttpRequestTool"]
