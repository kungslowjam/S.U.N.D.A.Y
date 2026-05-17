"""Academic search tools for papers and scholarly metadata."""

from __future__ import annotations

import json
import time
import urllib.parse
import xml.etree.ElementTree as ET
from typing import Any

import httpx

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

_HEADERS = {"User-Agent": "SUNDAY/0.1 academic-search (local user agent)"}


def _year_ok(year: int | None, start_year: int | None, end_year: int | None) -> bool:
    if year is None:
        return True
    if start_year is not None and year < start_year:
        return False
    if end_year is not None and year > end_year:
        return False
    return True


# ---------------------------------------------------------------------------
# Python fallback implementations
# ---------------------------------------------------------------------------

def _semantic_scholar_python(
    query: str,
    limit: int,
    start_year: int | None,
    end_year: int | None,
) -> ToolResult:
    fields = ",".join(
        [
            "title",
            "authors",
            "year",
            "abstract",
            "citationCount",
            "influentialCitationCount",
            "publicationVenue",
            "externalIds",
            "openAccessPdf",
            "url",
        ]
    )
    url = "https://api.semanticscholar.org/graph/v1/paper/search"
    request_limit = min(max(limit * 3, limit), 50)
    try:
        with httpx.Client(timeout=20.0) as client:
            resp = None
            for attempt in range(3):
                resp = client.get(
                    url,
                    params={
                        "query": query,
                        "limit": request_limit,
                        "fields": fields,
                    },
                    headers=_HEADERS,
                )
                if resp.status_code != 429:
                    break
                time.sleep(1.5 * (attempt + 1))
            assert resp is not None
            resp.raise_for_status()
    except Exception as exc:
        return ToolResult(
            tool_name="semantic_scholar_search",
            content=f"Semantic Scholar search failed: {exc}",
            success=False,
        )

    data = resp.json()
    papers = []
    for paper in data.get("data", []):
        year = paper.get("year")
        if not _year_ok(year, start_year, end_year):
            continue
        papers.append(paper)
        if len(papers) >= limit:
            break

    if not papers:
        return ToolResult(
            tool_name="semantic_scholar_search",
            content="No matching academic papers found.",
            success=True,
            metadata={"query": query, "num_results": 0},
        )

    lines = []
    for idx, paper in enumerate(papers, start=1):
        authors = ", ".join(
            author.get("name", "")
            for author in paper.get("authors", [])[:4]
            if author.get("name")
        )
        if len(paper.get("authors", [])) > 4:
            authors += ", et al."
        external = paper.get("externalIds") or {}
        doi = external.get("DOI")
        arxiv = external.get("ArXiv")
        venue = (paper.get("publicationVenue") or {}).get("name") or ""
        pdf = (paper.get("openAccessPdf") or {}).get("url") or ""
        abstract = (paper.get("abstract") or "").replace("\n", " ").strip()
        if len(abstract) > 450:
            abstract = abstract[:450] + "..."
        lines.extend(
            [
                f"{idx}. {paper.get('title', 'Untitled')}",
                f"   Year: {paper.get('year') or 'unknown'}"
                + (f" | Venue: {venue}" if venue else ""),
                f"   Authors: {authors or 'unknown'}",
                f"   Citations: {paper.get('citationCount', 0)}"
                f" | Influential: {paper.get('influentialCitationCount', 0)}",
                f"   URL: {paper.get('url') or ''}",
            ]
        )
        if doi:
            lines.append(f"   DOI: {doi}")
        if arxiv:
            lines.append(f"   arXiv: https://arxiv.org/abs/{arxiv}")
        if pdf:
            lines.append(f"   PDF: {pdf}")
        if abstract:
            lines.append(f"   Abstract: {abstract}")
        lines.append("")

    return ToolResult(
        tool_name="semantic_scholar_search",
        content="\n".join(lines).strip(),
        success=True,
        metadata={
            "query": query,
            "num_results": len(papers),
            "source": "semantic_scholar",
        },
    )


def _arxiv_python(
    query: str,
    limit: int,
    start_year: int | None,
    end_year: int | None,
) -> ToolResult:
    api_query = "+".join(urllib.parse.quote(part) for part in query.split())
    request_limit = min(max(limit * 3, limit), 50)
    url = (
        "https://export.arxiv.org/api/query"
        f"?search_query=all:{api_query}"
        f"&sortBy=submittedDate&sortOrder=descending&max_results={request_limit}"
    )

    try:
        with httpx.Client(timeout=20.0) as client:
            resp = None
            for attempt in range(3):
                resp = client.get(url, headers=_HEADERS)
                if resp.status_code != 429:
                    break
                time.sleep(3.2 * (attempt + 1))
            assert resp is not None
            resp.raise_for_status()
    except Exception as exc:
        return ToolResult(
            tool_name="arxiv_search",
            content=f"arXiv search failed: {exc}",
            success=False,
        )

    ns = {"a": "http://www.w3.org/2005/Atom"}
    root = ET.fromstring(resp.text)
    papers = []
    for entry in root.findall("a:entry", ns):
        published = (entry.findtext("a:published", default="", namespaces=ns) or "")
        year = int(published[:4]) if published[:4].isdigit() else None
        if not _year_ok(year, start_year, end_year):
            continue
        papers.append(entry)
        if len(papers) >= limit:
            break

    if not papers:
        return ToolResult(
            tool_name="arxiv_search",
            content="No matching arXiv papers found.",
            success=True,
            metadata={"query": query, "num_results": 0},
        )

    lines = []
    for idx, entry in enumerate(papers, start=1):
        title = (entry.findtext("a:title", default="", namespaces=ns) or "").strip()
        title = " ".join(title.split())
        arxiv_url = (entry.findtext("a:id", default="", namespaces=ns) or "").strip()
        arxiv_id = arxiv_url.split("/abs/")[-1]
        published = entry.findtext("a:published", default="", namespaces=ns)[:10]
        authors = ", ".join(
            author.findtext("a:name", default="", namespaces=ns)
            for author in entry.findall("a:author", ns)[:4]
        )
        if len(entry.findall("a:author", ns)) > 4:
            authors += ", et al."
        summary = (
            entry.findtext("a:summary", default="", namespaces=ns) or ""
        ).strip()
        summary = " ".join(summary.split())
        if len(summary) > 450:
            summary = summary[:450] + "..."
        lines.extend(
            [
                f"{idx}. {title}",
                f"   Published: {published}",
                f"   Authors: {authors or 'unknown'}",
                f"   URL: https://arxiv.org/abs/{arxiv_id}",
                f"   PDF: https://arxiv.org/pdf/{arxiv_id}",
                f"   Abstract: {summary}",
                "",
            ]
        )

    return ToolResult(
        tool_name="arxiv_search",
        content="\n".join(lines).strip(),
        success=True,
        metadata={"query": query, "num_results": len(papers), "source": "arxiv"},
    )


def _openalex_python(
    query: str,
    limit: int,
    start_year: int | None,
    end_year: int | None,
) -> ToolResult:
    filters = []
    if start_year is not None:
        filters.append(f"from_publication_date:{start_year}-01-01")
    if end_year is not None:
        filters.append(f"to_publication_date:{end_year}-12-31")

    params_out = {
        "search": query,
        "per-page": limit,
        "sort": "relevance_score:desc",
        "mailto": "local@sunday.local",
    }
    if filters:
        params_out["filter"] = ",".join(filters)

    try:
        with httpx.Client(timeout=20.0) as client:
            resp = client.get(
                "https://api.openalex.org/works",
                params=params_out,
                headers=_HEADERS,
            )
            resp.raise_for_status()
    except Exception as exc:
        return ToolResult(
            tool_name="openalex_search",
            content=f"OpenAlex search failed: {exc}",
            success=False,
        )

    data = resp.json()
    works = data.get("results", [])[:limit]
    if not works:
        return ToolResult(
            tool_name="openalex_search",
            content="No matching OpenAlex papers found.",
            success=True,
            metadata={"query": query, "num_results": 0},
        )

    def _abstract(inv: dict[str, list[int]] | None) -> str:
        if not inv:
            return ""
        positions: list[tuple[int, str]] = []
        for word, indexes in inv.items():
            for idx in indexes:
                positions.append((idx, word))
        text = " ".join(word for _, word in sorted(positions))
        return text[:450] + "..." if len(text) > 450 else text

    lines = []
    for idx, work in enumerate(works, start=1):
        authorships = work.get("authorships") or []
        authors = ", ".join(
            (a.get("author") or {}).get("display_name", "")
            for a in authorships[:4]
            if (a.get("author") or {}).get("display_name")
        )
        if len(authorships) > 4:
            authors += ", et al."
        doi = work.get("doi") or ""
        host = (work.get("primary_location") or {}).get("source") or {}
        venue = host.get("display_name") or ""
        oa_url = (work.get("open_access") or {}).get("oa_url") or ""
        landing = work.get("id") or ""
        abstract = _abstract(work.get("abstract_inverted_index"))
        lines.extend(
            [
                f"{idx}. {work.get('title') or 'Untitled'}",
                f"   Year: {work.get('publication_year') or 'unknown'}"
                + (f" | Venue: {venue}" if venue else ""),
                f"   Authors: {authors or 'unknown'}",
                f"   Citations: {work.get('cited_by_count', 0)}",
                f"   URL: {doi or landing}",
            ]
        )
        if oa_url and oa_url != doi:
            lines.append(f"   Open access: {oa_url}")
        if abstract:
            lines.append(f"   Abstract: {abstract}")
        lines.append("")

    return ToolResult(
        tool_name="openalex_search",
        content="\n".join(lines).strip(),
        success=True,
        metadata={"query": query, "num_results": len(works), "source": "openalex"},
    )


# ---------------------------------------------------------------------------
# Tool classes — Rust-first with Python fallback
# ---------------------------------------------------------------------------

@ToolRegistry.register("semantic_scholar_search")
class SemanticScholarSearchTool(BaseTool):
    """Search Semantic Scholar for academic papers."""

    tool_id = "semantic_scholar_search"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="semantic_scholar_search",
            description=(
                "Search Semantic Scholar for academic papers. Use this before "
                "generic web_search when the user asks for research papers, "
                "literature, DOI, citations, authors, or publication years."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Academic paper search query.",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of papers to return.",
                    },
                    "start_year": {
                        "type": "integer",
                        "description": "Optional earliest publication year.",
                    },
                    "end_year": {
                        "type": "integer",
                        "description": "Optional latest publication year.",
                    },
                },
                "required": ["query"],
            },
            category="academic_search",
            timeout_seconds=20.0,
        )

    def execute(self, **params: Any) -> ToolResult:
        query = str(params.get("query") or "").strip()
        if not query:
            return ToolResult(
                tool_name=self.tool_id,
                content="No query provided.",
                success=False,
            )
        limit = max(1, min(int(params.get("limit") or 5), 20))
        start_year = params.get("start_year")
        end_year = params.get("end_year")
        start_year = int(start_year) if start_year else None
        end_year = int(end_year) if end_year else None

        # Prefer Rust backend
        try:
            from sunday._rust_bridge import get_rust_module

            _rust = get_rust_module()
            content = _rust.SemanticScholarSearchTool().execute(
                query, limit, start_year, end_year
            )
            return ToolResult(
                tool_name=self.tool_id,
                content=content,
                success=True,
                metadata={
                    "query": query,
                    "num_results": "unknown",
                    "source": "semantic_scholar",
                },
            )
        except ImportError:
            pass
        except Exception as exc:
            return ToolResult(
                tool_name=self.tool_id,
                content=f"Rust semantic_scholar_search failed: {exc}",
                success=False,
            )

        return _semantic_scholar_python(query, limit, start_year, end_year)


@ToolRegistry.register("arxiv_search")
class ArxivSearchTool(BaseTool):
    """Search arXiv directly via the public Atom API."""

    tool_id = "arxiv_search"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="arxiv_search",
            description=(
                "Search arXiv for academic preprints. Use only when the user "
                "explicitly asks for arXiv/preprints, or when openalex_search "
                "returns no useful results. For broad research paper searches "
                "such as water management, smart water, IoT water monitoring, "
                "or applied engineering papers, prefer openalex_search first."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "arXiv query."},
                    "limit": {
                        "type": "integer",
                        "description": "Maximum papers to return.",
                    },
                    "start_year": {
                        "type": "integer",
                        "description": "Optional earliest publication year.",
                    },
                    "end_year": {
                        "type": "integer",
                        "description": "Optional latest publication year.",
                    },
                },
                "required": ["query"],
            },
            category="academic_search",
            timeout_seconds=20.0,
        )

    def execute(self, **params: Any) -> ToolResult:
        query = str(params.get("query") or "").strip()
        if not query:
            return ToolResult(
                tool_name=self.tool_id,
                content="No query provided.",
                success=False,
            )
        limit = max(1, min(int(params.get("limit") or 5), 20))
        start_year = params.get("start_year")
        end_year = params.get("end_year")
        start_year = int(start_year) if start_year else None
        end_year = int(end_year) if end_year else None

        # Prefer Rust backend
        try:
            from sunday._rust_bridge import get_rust_module

            _rust = get_rust_module()
            content = _rust.ArxivSearchTool().execute(
                query, limit, start_year, end_year
            )
            return ToolResult(
                tool_name=self.tool_id,
                content=content,
                success=True,
                metadata={
                    "query": query,
                    "num_results": "unknown",
                    "source": "arxiv",
                },
            )
        except ImportError:
            pass
        except Exception as exc:
            return ToolResult(
                tool_name=self.tool_id,
                content=f"Rust arxiv_search failed: {exc}",
                success=False,
            )

        return _arxiv_python(query, limit, start_year, end_year)


@ToolRegistry.register("openalex_search")
class OpenAlexSearchTool(BaseTool):
    """Search OpenAlex for scholarly works."""

    tool_id = "openalex_search"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="openalex_search",
            description=(
                "Search OpenAlex for scholarly papers and metadata. This is a "
                "good fallback or primary tool for research-paper requests when "
                "Semantic Scholar is rate-limited."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Paper search query."},
                    "limit": {
                        "type": "integer",
                        "description": "Maximum papers to return.",
                    },
                    "start_year": {
                        "type": "integer",
                        "description": "Optional earliest publication year.",
                    },
                    "end_year": {
                        "type": "integer",
                        "description": "Optional latest publication year.",
                    },
                },
                "required": ["query"],
            },
            category="academic_search",
            timeout_seconds=20.0,
        )

    def execute(self, **params: Any) -> ToolResult:
        query = str(params.get("query") or "").strip()
        if not query:
            return ToolResult(
                tool_name=self.tool_id,
                content="No query provided.",
                success=False,
            )
        limit = max(1, min(int(params.get("limit") or 5), 20))
        start_year = params.get("start_year")
        end_year = params.get("end_year")
        start_year = int(start_year) if start_year else None
        end_year = int(end_year) if end_year else None

        # Prefer Rust backend
        try:
            from sunday._rust_bridge import get_rust_module

            _rust = get_rust_module()
            content = _rust.OpenAlexSearchTool().execute(
                query, limit, start_year, end_year
            )
            return ToolResult(
                tool_name=self.tool_id,
                content=content,
                success=True,
                metadata={
                    "query": query,
                    "num_results": "unknown",
                    "source": "openalex",
                },
            )
        except ImportError:
            pass
        except Exception as exc:
            return ToolResult(
                tool_name=self.tool_id,
                content=f"Rust openalex_search failed: {exc}",
                success=False,
            )

        return _openalex_python(query, limit, start_year, end_year)


__all__ = ["SemanticScholarSearchTool", "ArxivSearchTool", "OpenAlexSearchTool"]
