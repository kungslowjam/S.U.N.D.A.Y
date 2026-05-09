from __future__ import annotations

import argparse
import asyncio
import base64
import json
import re
import sys
import tempfile
import threading
import urllib.error
import urllib.request
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parent
DEFAULT_LLM_ENDPOINT = "http://127.0.0.1:8081/v1/chat/completions"
VOICE_LLM_ENDPOINT = "http://127.0.0.1:8082/v1/chat/completions"
SUNDAY_AGENT_ENDPOINT = "http://127.0.0.1:8000/v1/voice/turn"
SENTENCE_RE = re.compile(
    r"(.+?[.!?](?:\s|$)|.+?[。！？](?:\s|$)|.+?(?:ครับ|ค่ะ|คะ|นะ)(?:\s|$))",
    re.S,
)
THAI_RE = re.compile(r"[\u0E00-\u0E7F]")
_WHISPER_MODEL = None
_WHISPER_MODEL_NAME = None
_WHISPER_LOADING = False
_WHISPER_LOCK = threading.Lock()

TECH_TERM_REPLACEMENTS = (
    (re.compile(r"เอ็กซ์\s*เพลน", re.I), "explain"),
    (re.compile(r"อธิบาย\s*architecture", re.I), "อธิบาย architecture"),
    (re.compile(r"อาร์\s*คิ\s*เท็?ค\s*เจอ(?:ร์)?|อา\s*คิ?ว\s*เท็?ก(?:เจอ|เจอร์)?", re.I), "architecture"),
    (re.compile(r"วอยซ์\s*โอ\s*เวอร์|วอย\s*โอ\s*เวอร์", re.I), "voice overlay"),
    (re.compile(r"เจมิไน\s*ไลฟ์|เจมินี\s*ไลฟ์", re.I), "Gemini Live"),
    (re.compile(r"โมเดล", re.I), "model"),
    (re.compile(r"ฟรอนท์\s*เอนด์|ฟร้อนท์\s*เอนด์", re.I), "frontend"),
    (re.compile(r"แบ็ค\s*เอนด์|แบ็ก\s*เอนด์", re.I), "backend"),
    (re.compile(r"เซิร์ฟเวอร์|เซิฟเวอร์", re.I), "server"),
    (re.compile(r"สตรีม(?:มิ่ง)?", re.I), "streaming"),
    (re.compile(r"เลเทนซี|ลาเทนซี", re.I), "latency"),
    (re.compile(r"ทรานส์ไคร(?:บ์|บ)", re.I), "transcribe"),
    (re.compile(r"เอส\s*ที\s*ที", re.I), "STT"),
    (re.compile(r"ที\s*ที\s*เอส", re.I), "TTS"),
    (re.compile(r"แอล\s*แอล\s*เอ็ม", re.I), "LLM"),
    (re.compile(r"เชลล์|เชล", re.I), "shell"),
    (re.compile(r"เทอร์มินัล|เทอมินอล", re.I), "terminal"),
    (re.compile(r"โฟลเดอร์|โฟลเด้อ", re.I), "folder"),
    (re.compile(r"ไดเรกทอรี|ไดเรกทอรี่", re.I), "directory"),
    (re.compile(r"เมค\s*ได", re.I), "mkdir"),
    (re.compile(r"กูเกิล|กูเกิ้ล", re.I), "google"),
    (re.compile(r"สคริปต์|สคริป", re.I), "script"),
)


def _set_cors(handler: SimpleHTTPRequestHandler) -> None:
    handler.send_header("Access-Control-Allow-Origin", "*")
    handler.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    handler.send_header("Access-Control-Allow-Headers", "Content-Type, X-STT-Model, X-STT-Language")

def _json_response(handler: SimpleHTTPRequestHandler, status: int, payload: dict) -> None:
    body = json.dumps(payload).encode("utf-8")
    try:
        handler.send_response(status)
        _set_cors(handler)
        handler.send_header("Content-Type", "application/json; charset=utf-8")
        handler.send_header("Content-Length", str(len(body)))
        handler.end_headers()
        handler.wfile.write(body)
    except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
        return


def _ndjson_write(handler: SimpleHTTPRequestHandler, payload: dict) -> None:
    line = json.dumps(payload, ensure_ascii=False).encode("utf-8") + b"\n"
    try:
        handler.wfile.write(line)
        handler.wfile.flush()
    except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
        raise BrokenPipeError()


def _iter_sse_json(response) -> str:
    for raw in response:
        line = raw.decode("utf-8", errors="ignore").strip()
        if not line.startswith("data: "):
            continue
        data = line[6:].strip()
        if not data or data == "[DONE]":
            continue
        try:
            payload = json.loads(data)
        except json.JSONDecodeError:
            continue
        delta_obj = payload.get("choices", [{}])[0].get("delta", {})
        delta = delta_obj.get("content") or ""
        if isinstance(delta, str) and delta:
            yield delta


def _pop_sentence(buffer: str, force: bool = False) -> tuple[str | None, str]:
    match = SENTENCE_RE.match(buffer)
    if match and len(match.group(1).strip()) >= 14:
        text = match.group(1).strip()
        return text, buffer[match.end() :]
    if force and buffer.strip():
        return buffer.strip(), ""
    if len(buffer) >= 88:
        idx = max(buffer.rfind(","), buffer.rfind(";"), buffer.rfind(":"), buffer.rfind(" "), buffer.rfind("，"))
        if idx > 28:
            return buffer[: idx + 1].strip(), buffer[idx + 1 :]
    return None, buffer


async def _edge_tts_mp3(text: str, voice: str) -> bytes:
    try:
        import edge_tts
    except ImportError as exc:
        raise RuntimeError(
            "edge-tts is not installed. Run: py -m pip install edge-tts"
        ) from exc

    communicate = edge_tts.Communicate(text, voice=voice, rate="+4%")
    audio = bytearray()
    async for chunk in communicate.stream():
        if chunk["type"] == "audio":
            audio.extend(chunk["data"])
    return bytes(audio)


def _tts_mp3(text: str, voice: str) -> bytes:
    return asyncio.run(_edge_tts_mp3(text, voice))


class VoiceLiveHandler(SimpleHTTPRequestHandler):
    server_version = "SundayVoiceLive/0.2"

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def do_OPTIONS(self) -> None:
        self.send_response(200)
        _set_cors(self)
        self.end_headers()

    def do_POST(self) -> None:
        if self.path == "/api/transcribe":
            self._handle_transcribe()
            return
        if self.path != "/api/live-turn":
            _json_response(self, 404, {"error": "not found"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            request = json.loads(self.rfile.read(length).decode("utf-8"))
        except Exception as exc:
            _json_response(self, 400, {"error": f"bad request: {exc}"})
            return

        endpoint = self._choose_endpoint(request)
        voice = request.get("voice") or "auto"
        if endpoint.rstrip("/").endswith("/v1/voice/turn"):
            self._handle_sunday_voice_turn(endpoint, request, voice)
            return

        llm_body = {
            "model": request.get("model") or "local-model",
            "messages": self._normalize_messages(request.get("messages") or []),
            "stream": True,
            "temperature": request.get("temperature", 0.4),
            "top_p": request.get("top_p", 0.82),
            "max_tokens": request.get("max_tokens", 160),
            "chat_template_kwargs": {"enable_thinking": False},
        }

        self.send_response(200)
        _set_cors(self)
        self.send_header("Content-Type", "application/x-ndjson; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()

        req = urllib.request.Request(
            endpoint,
            data=json.dumps(llm_body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        full_text = ""
        tts_buffer = ""
        try:
            with urllib.request.urlopen(req, timeout=180) as response:
                for delta in _iter_sse_json(response):
                    full_text += delta
                    tts_buffer += delta
                    _ndjson_write(self, {"type": "text_delta", "delta": delta})

                    while True:
                        sentence, tts_buffer = _pop_sentence(tts_buffer)
                        if not sentence:
                            break
                        self._emit_audio(sentence, voice)

            sentence, tts_buffer = _pop_sentence(tts_buffer, force=True)
            if sentence:
                self._emit_audio(sentence, voice)
            _ndjson_write(self, {"type": "done", "text": full_text})
        except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
            return
        except urllib.error.URLError as exc:
            try:
                _ndjson_write(self, {"type": "error", "error": f"LLM request failed: {exc}"})
            except BrokenPipeError:
                return
        except Exception as exc:
            try:
                _ndjson_write(self, {"type": "error", "error": str(exc)})
            except BrokenPipeError:
                return

    def _handle_sunday_voice_turn(self, endpoint: str, request: dict, voice: str) -> None:
        messages = request.get("messages") or []
        text = ""
        for message in reversed(messages):
            if message.get("role") == "user":
                text = str(message.get("content") or "").strip()
                break

        self.send_response(200)
        _set_cors(self)
        self.send_header("Content-Type", "application/x-ndjson; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()

        body = {
            "text": text,
            "messages": messages[-12:],
            "model": request.get("model") or "local-model",
            "temperature": request.get("temperature", 0.4),
            "max_tokens": request.get("max_tokens", 160),
            "stream": True, # Enable streaming from agent
        }
        req = urllib.request.Request(
            endpoint,
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        full_text = ""
        tts_buffer = ""
        try:
            with urllib.request.urlopen(req, timeout=300) as response:
                # Handle potential streaming response from agent
                if response.info().get("Content-Type", "").startswith("application/x-ndjson"):
                    for line in response:
                        if not line.strip(): continue
                        try:
                            ev = json.loads(line.decode("utf-8"))
                            if ev.get("event") == "text_delta":
                                delta = ev.get("text", "")
                                full_text += delta
                                tts_buffer += delta
                                _ndjson_write(self, {"type": "text_delta", "delta": delta})
                                while True:
                                    sentence, tts_buffer = _pop_sentence(tts_buffer)
                                    if not sentence: break
                                    self._emit_audio(sentence, voice)
                            elif ev.get("event") in ("tool_call_start", "skill_execute_start"):
                                name = ev.get("tool") or ev.get("skill") or "tool"
                                _ndjson_write(self, {"type": "status", "message": f"Using {name}..."})
                        except: continue
                else:
                    # Fallback for non-streaming
                    payload = json.loads(response.read().decode("utf-8"))
                    full_text = str(payload.get("text") or "").strip()
                    if full_text:
                        _ndjson_write(self, {"type": "text_delta", "delta": full_text})
                        for sentence in _split_for_tts(full_text):
                            self._emit_audio(sentence, voice)

            sentence, tts_buffer = _pop_sentence(tts_buffer, force=True)
            if sentence:
                self._emit_audio(sentence, voice)
            _ndjson_write(self, {"type": "done", "text": full_text})
        except Exception as exc:
            try: _ndjson_write(self, {"type": "error", "error": str(exc)})
            except: pass

    def _handle_transcribe(self) -> None:
        try:
            content_type = self.headers.get("Content-Type", "")
            length = int(self.headers.get("Content-Length", "0"))
            audio = self.rfile.read(length)
            if not audio:
                _json_response(self, 400, {"error": "empty audio"})
                return

            suffix = ".webm"
            if "wav" in content_type:
                suffix = ".wav"
            elif "mp4" in content_type:
                suffix = ".mp4"

            with tempfile.NamedTemporaryFile(delete=False, suffix=suffix) as tmp:
                tmp.write(audio)
                tmp_path = Path(tmp.name)
            try:
                model_name = self.headers.get("X-STT-Model", "tiny")
                language = self.headers.get("X-STT-Language", "auto")
                text = _transcribe_audio(tmp_path, model_name=model_name, language=language)
            finally:
                tmp_path.unlink(missing_ok=True)
            self.send_response(200)
            _set_cors(self)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.end_headers()
            self.wfile.write(json.dumps({"text": text}).encode("utf-8"))
        except Exception as exc:
            _json_response(self, 500, {"error": str(exc)})

    def _emit_audio(self, text: str, voice: str) -> None:
        try:
            selected_voice = _select_voice(text, voice)
            audio = _tts_mp3(text, selected_voice)
            _ndjson_write(
                self,
                {
                    "type": "audio",
                    "text": text,
                    "mime": "audio/mpeg",
                    "voice": selected_voice,
                    "audio": base64.b64encode(audio).decode("ascii"),
                },
            )
        except Exception as exc:
            _ndjson_write(self, {"type": "tts_error", "text": text, "error": str(exc)})

    @staticmethod
    def _normalize_messages(messages: list[dict]) -> list[dict]:
        if not messages:
            return messages
        system = {
            "role": "system",
            "content": (
                "You are SUNDAY Voice in fast live conversation mode. "
                "Reply only with the final answer. "
                "If the user speaks Thai, reply in Thai. "
                "If the user mixes Thai and English, reply in the same Thai-English mix. "
                "Keep English technical terms when they are natural. "
                "Do not repeat the user's words. Do not explain your thinking. "
                "Keep replies under 25 words unless the user asks for details. "
                "Prefer one natural spoken sentence."
            ),
        }
        cleaned = []
        for message in messages:
            role = message.get("role")
            content = str(message.get("content", "")).replace("/no_think", "").strip()
            if not content:
                continue
            if role == "system":
                continue
            cleaned.append({"role": role, "content": content})
        return [system, *cleaned[-6:]]

    @staticmethod
    def _choose_endpoint(request: dict) -> str:
        route_mode = str(request.get("route_mode") or "").lower()
        primary = request.get("llm_endpoint") or VOICE_LLM_ENDPOINT
        fallback = request.get("fallback_endpoint") or SUNDAY_AGENT_ENDPOINT
        if route_mode == "live":
            return primary
        if route_mode == "agent":
            return fallback

        text = " ".join(
            str(message.get("content", ""))
            for message in request.get("messages", [])
            if message.get("role") == "user"
        ).lower()
        if primary.rstrip("/").endswith("/v1/voice/turn"):
            return primary
        agent_markers = (
            "tool", "tools", "skill", "skills", "research", "paper", "papers", "search", "browse",
            "web", "github", "code", "debug", "file", "folder", "open ", "install", "download",
            "run ", "execute", "shell", "terminal", "command", "mkdir", "delete", "remove", "update",
            "สรุป", "วิจัย", "ใช้ tool", "ใช้ skill", "ค้น", "ค้นหา", "หา research", "งานวิจัย", "เปิดไฟล์",
            "แก้ไฟล์", "ติดตั้ง", "ดาวน์โหลด", "รัน", "ประมวลผล", "สร้าง", "ลบ", "ย้าย", "ก๊อป", "ก๊อปปี้",
            "เขียนโค้ด", "แก้โค้ด", "ดิบัก", "เช็ค", "ตรวจสอบ", "เทอร์มินัล", "เชลล์", "คำสั่ง", "โฟลเดอร์",
            "ไดเรกทอรี", "เบราว์เซอร์", "กูเกิล", "เสิร์ช"
        )
        if request.get("force_fallback"):
            return fallback
        if any(marker in text for marker in agent_markers):
            return fallback
        return primary


def _select_voice(text: str, requested: str) -> str:
    if requested and requested != "auto":
        return requested
    thai_chars = len(THAI_RE.findall(text))
    alpha_chars = len(re.findall(r"[A-Za-z]", text))
    if thai_chars > 0 and thai_chars >= max(2, alpha_chars // 2):
        return "th-TH-PremwadeeNeural"
    return "en-US-AvaNeural"


def _split_for_tts(text: str) -> list[str]:
    chunks = []
    buffer = text.strip()
    while buffer:
        sentence, buffer = _pop_sentence(buffer, force=len(buffer) <= 140)
        if sentence:
            chunks.append(sentence)
        else:
            chunks.append(buffer.strip())
            break
    return chunks


def _transcribe_audio(path: Path, model_name: str = "tiny", language: str = "auto") -> str:
    global _WHISPER_MODEL, _WHISPER_MODEL_NAME, _WHISPER_LOADING
    try:
        from faster_whisper import WhisperModel
    except ImportError as exc:
        raise RuntimeError(
            "faster-whisper is not installed. Run: py -m pip install --user faster-whisper"
        ) from exc

    with _WHISPER_LOCK:
        if _WHISPER_MODEL is None or _WHISPER_MODEL_NAME != model_name:
            _WHISPER_LOADING = True
            
            # Local model detection
            target_model = model_name
            if model_name == "Vinxscribe/biodatlab-whisper-th-medium-faster":
                local_path = ROOT / "stt_models" / "thai-medium"
                if local_path.exists():
                    target_model = str(local_path)
            elif model_name == "pariya47/distill-whisper-th-large-v3-ct2":
                local_path = ROOT / "stt_models" / "distill-large"
                if local_path.exists():
                    target_model = str(local_path)

            device = "cuda"
            compute_type = "int8_float16"
            try:
                _WHISPER_MODEL = WhisperModel(target_model, device=device, compute_type=compute_type)
            except Exception:
                _WHISPER_MODEL = WhisperModel(target_model, device="cpu", compute_type="int8")
            _WHISPER_MODEL_NAME = model_name
            _WHISPER_LOADING = False

    forced_language = None if language == "auto" else language
    if "whisper-th" in model_name or "distill-whisper-th" in model_name or "biodatlab-whisper-th" in model_name:
        forced_language = "th"
    if model_name.endswith(".en"):
        forced_language = "en"

    segments, _ = _WHISPER_MODEL.transcribe(
        str(path),
        beam_size=10,
        vad_filter=True,
        language=forced_language,
        initial_prompt="สวัสดี SUNDAY ระบบช่วยงานอัจฉริยะ คุยภาษาไทยและอังกฤษ technical terms: architecture, shell, script, deploy",
        condition_on_previous_text=False,
        no_speech_threshold=0.2,
        word_timestamps=False,
    )
    text = " ".join(segment.text.strip() for segment in segments).strip()
    return _normalize_transcript(text)


def _normalize_transcript(text: str) -> str:
    normalized = text
    for pattern, replacement in TECH_TERM_REPLACEMENTS:
        normalized = pattern.sub(replacement, normalized)
    return re.sub(r"\s{2,}", " ", normalized).strip()


def _warm_stt_model(model_name: str) -> None:
    global _WHISPER_MODEL, _WHISPER_MODEL_NAME, _WHISPER_LOADING
    if _WHISPER_MODEL is not None and _WHISPER_MODEL_NAME == model_name:
        return
    try:
        from faster_whisper import WhisperModel

        _WHISPER_LOADING = True
        with _WHISPER_LOCK:
            try:
                _WHISPER_MODEL = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
            except Exception:
                _WHISPER_MODEL = WhisperModel(model_name, device="cpu", compute_type="int8")
            _WHISPER_MODEL_NAME = model_name
    except Exception as exc:
        print(f"STT warmup skipped: {exc}", flush=True)
    finally:
        _WHISPER_LOADING = False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8098)
    parser.add_argument("--preload-stt", default="tiny")
    args = parser.parse_args()

    if args.preload_stt:
        print(f"Warming faster-whisper {args.preload_stt} in background...", flush=True)
        threading.Thread(target=_warm_stt_model, args=(args.preload_stt,), daemon=True).start()

    server = ThreadingHTTPServer((args.host, args.port), VoiceLiveHandler)
    print(f"SUNDAY Voice Live serving http://{args.host}:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
