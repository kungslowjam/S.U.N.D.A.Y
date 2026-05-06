"""Speech API routes for browser microphone transcription."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, File, Request, UploadFile

router = APIRouter(prefix="/v1/speech", tags=["speech"])


def _audio_format(filename: str, content_type: str) -> str:
    suffix = Path(filename).suffix.lower().lstrip(".")
    if suffix:
        return suffix
    if "/" in content_type:
        return content_type.split("/", 1)[1].split(";", 1)[0]
    return "webm"


@router.get("/health")
async def speech_health(request: Request) -> dict:
    """Return whether a server-side speech backend is available."""
    backend = getattr(request.app.state, "speech_backend", None)
    if backend is None:
        return {
            "available": False,
            "reason": "No server speech backend configured; browser fallback may be used.",
        }

    try:
        ok = backend.health()
    except Exception as exc:
        return {
            "available": False,
            "backend": getattr(backend, "backend_id", ""),
            "reason": str(exc),
        }

    return {
        "available": bool(ok),
        "backend": getattr(backend, "backend_id", ""),
        "reason": "" if ok else "Speech backend is not healthy.",
    }


@router.post("/transcribe")
async def transcribe_audio(
    request: Request,
    file: UploadFile = File(...),
) -> dict:
    """Transcribe uploaded browser audio through the configured backend."""
    backend = getattr(request.app.state, "speech_backend", None)
    if backend is None:
        return {
            "text": "",
            "language": None,
            "confidence": None,
            "duration_seconds": 0,
        }

    audio = await file.read()
    config = getattr(request.app.state, "config", None)
    language = getattr(getattr(config, "speech", None), "language", "") or None
    result = backend.transcribe(
        audio,
        format=_audio_format(file.filename or "", file.content_type or ""),
        language=language,
    )
    return {
        "text": result.text,
        "language": result.language,
        "confidence": result.confidence,
        "duration_seconds": result.duration_seconds,
    }


__all__ = ["router"]
