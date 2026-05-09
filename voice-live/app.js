const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
const DEFAULT_VOICE_LLM_ENDPOINT = "http://127.0.0.1:8082/v1/chat/completions";
const DEFAULT_AGENT_ENDPOINT = "http://127.0.0.1:8000/v1/voice/turn";
const SETTINGS_VERSION = "fast-voice-v5";
const DEFAULT_STT_MODEL = "pariya47/distill-whisper-th-large-v3-ct2";

const state = {
  live: false,
  recognizing: false,
  speaking: false,
  stoppingForAssistant: false,
  turnCount: 0,
  aborter: null,
  speechQueue: [],
  currentAudio: null,
  routeMode: "live",
  mediaStream: null,
  mediaRecorder: null,
  recordingChunks: [],
  audioContext: null,
  analyser: null,
  micLevelTimer: null,
  silenceTimer: null,
  recording: false,
  localSpeechActive: false,
  localTranscribing: false,
  recordingStartedAt: 0,
  lastUtteranceStartedAt: 0,
  lastSttStartedAt: 0,
  lastSttEndedAt: 0,
  history: [
    {
      role: "system",
      content:
        "You are SUNDAY, a concise bilingual Thai-English voice assistant. If the user speaks Thai, reply in Thai. If the user mixes Thai and English, reply in the same Thai-English mix. Keep English technical terms when natural. Answer directly in short spoken sentences. Do not show reasoning, thinking, analysis, markdown, or hidden chain-of-thought.",
    },
  ],
};

const els = {
  status: document.getElementById("status"),
  meter: document.getElementById("meterFill"),
  toggle: document.getElementById("toggleBtn"),
  talk: document.getElementById("talkBtn"),
  interrupt: document.getElementById("interruptBtn"),
  mode: document.getElementById("modeInput"),
  stt: document.getElementById("sttInput"),
  sttModel: document.getElementById("sttModelInput"),
  sttLang: document.getElementById("sttLangInput"),
  endpoint: document.getElementById("endpointInput"),
  fallbackEndpoint: document.getElementById("fallbackEndpointInput"),
  model: document.getElementById("modelInput"),
  voice: document.getElementById("voiceInput"),
  conversation: document.getElementById("conversation"),
  latency: document.getElementById("latencyText"),
  turn: document.getElementById("turnText"),
  liveMode: document.getElementById("liveModeBtn"),
  agentMode: document.getElementById("agentModeBtn"),
  routeHint: document.getElementById("routeHint"),
  settingsToggle: document.getElementById("settingsToggleBtn"),
  settingsDrawer: document.getElementById("settingsDrawer"),
  closeSettings: document.getElementById("closeSettingsBtn"),
  drawerOverlay: document.getElementById("drawerOverlay"),
};

function toggleDrawer() {
  const isOpen = els.settingsDrawer.classList.contains("open");
  if (isOpen) {
    els.settingsDrawer.classList.remove("open");
    els.drawerOverlay.classList.remove("open");
  } else {
    els.settingsDrawer.classList.add("open");
    els.drawerOverlay.classList.add("open");
  }
}

els.settingsToggle.addEventListener("click", toggleDrawer);
els.closeSettings.addEventListener("click", toggleDrawer);
els.drawerOverlay.addEventListener("click", toggleDrawer);


let recognition = null;
let finalText = "";
let interimText = "";
let silenceTimer = null;
let userNode = null;
let maxRecordTimer = null;
let vadCooldownUntil = 0;

const MODES = {
  quick: {
    silenceMs: 420,
    maxTokens: 64,
    temperature: 0.35,
    topP: 0.78,
    minSpeakChars: 28,
    speechRate: 1.08,
    startThreshold: 0.036,
    stopThreshold: 0.018,
    minRecordMs: 620,
    maxRecordMs: 3800,
  },
  balanced: {
    silenceMs: 520,
    maxTokens: 80,
    temperature: 0.45,
    topP: 0.82,
    minSpeakChars: 36,
    speechRate: 1.03,
    startThreshold: 0.036,
    stopThreshold: 0.018,
    minRecordMs: 760,
    maxRecordMs: 4800,
  },
};

function setStatus(text) {
  els.status.textContent = text;
}

function settings() {
  return MODES[els.mode.value] || MODES.balanced;
}

function requestOptions() {
  const base = settings();
  if (state.routeMode === "agent") {
    return {
      ...base,
      maxTokens: Math.max(base.maxTokens, 220),
      temperature: 0.35,
      topP: 0.78,
    };
  }
  return base;
}

function setRouteMode(mode, persist = true) {
  state.routeMode = mode === "agent" ? "agent" : "live";
  els.liveMode.classList.toggle("active", state.routeMode === "live");
  els.agentMode.classList.toggle("active", state.routeMode === "agent");
  els.endpoint.value = state.routeMode === "live" ? DEFAULT_VOICE_LLM_ENDPOINT : DEFAULT_AGENT_ENDPOINT;
  els.fallbackEndpoint.value = state.routeMode === "live" ? DEFAULT_AGENT_ENDPOINT : DEFAULT_VOICE_LLM_ENDPOINT;
  els.routeHint.textContent =
    state.routeMode === "live"
      ? "Fast voice model. Best for realtime Thai/English conversation."
      : "SUNDAY agent path. Slower, but tools, skills, memory, and data sources are enabled.";
  if (persist) saveSettings();
}

function saveSettings() {
  const values = {
    routeMode: state.routeMode,
    mode: els.mode.value,
    stt: els.stt.value,
    sttModel: els.sttModel.value,
    sttLang: els.sttLang.value,
    endpoint: els.endpoint.value,
    fallbackEndpoint: els.fallbackEndpoint.value,
    model: els.model.value,
    voice: els.voice.value,
    settingsVersion: SETTINGS_VERSION,
  };
  localStorage.setItem("sundayVoiceLiveSettings", JSON.stringify(values));
}

function loadSettings() {
  try {
    const values = JSON.parse(localStorage.getItem("sundayVoiceLiveSettings") || "{}");
    for (const [key, value] of Object.entries(values)) {
      if (key === "routeMode") continue;
      const input = {
        mode: els.mode,
        stt: els.stt,
        sttModel: els.sttModel,
        sttLang: els.sttLang,
        endpoint: els.endpoint,
        fallbackEndpoint: els.fallbackEndpoint,
        model: els.model,
        voice: els.voice,
      }[key];
      if (input && value) input.value = value;
    }
    if (
      values.settingsVersion !== SETTINGS_VERSION ||
      /127\.0\.0\.1:8000\/v1\/voice\/turn/.test(els.endpoint.value)
    ) {
      els.endpoint.value = DEFAULT_VOICE_LLM_ENDPOINT;
      els.fallbackEndpoint.value = DEFAULT_AGENT_ENDPOINT;
      els.sttModel.value = DEFAULT_STT_MODEL;
      els.mode.value = "quick";
      saveSettings();
    }
    setRouteMode(values.routeMode === "agent" ? "agent" : "live", false);
  } catch {}
}

function addTurn(role, text = "") {
  const node = document.createElement("div");
  node.className = `turn ${role}`;
  node.textContent = text;
  els.conversation.appendChild(node);
  els.conversation.scrollTop = els.conversation.scrollHeight;
  return node;
}

function stopSpeech() {
  window.speechSynthesis.cancel();
  state.speechQueue = [];
  if (state.currentAudio) {
    state.currentAudio.pause();
    state.currentAudio.src = "";
    state.currentAudio = null;
  }
  state.speaking = false;
  els.interrupt.disabled = true;
}

function interrupt() {
  stopSpeech();
  if (state.aborter) {
    state.aborter.abort();
    state.aborter = null;
  }
  setStatus(state.live ? "Listening" : "Idle");
}

function isIntentionalAbort(error) {
  const message = String(error?.message || error || "").toLowerCase();
  return (
    error?.name === "AbortError" ||
    message.includes("bodystreambuffer was aborted") ||
    message.includes("body stream buffer was aborted") ||
    message.includes("operation was aborted") ||
    message.includes("signal is aborted")
  );
}

function speakAudio(base64Audio, mime = "audio/mpeg") {
  if (!base64Audio) return;
  state.speechQueue.push({ base64Audio, mime });
  if (!state.speaking) playNextSpeech();
}

function playNextSpeech() {
  const item = state.speechQueue.shift();
  if (!item) {
    state.speaking = false;
    els.interrupt.disabled = true;
    if (state.live) resumeRecognition();
    setStatus(state.live ? "Listening" : "Idle");
    return;
  }

  pauseRecognitionForAssistant();
  const audio = new Audio(`data:${item.mime};base64,${item.base64Audio}`);
  state.currentAudio = audio;
  state.speaking = true;
  els.interrupt.disabled = false;
  setStatus("Speaking");
  audio.onended = () => {
    state.currentAudio = null;
    playNextSpeech();
  };
  audio.onerror = () => {
    state.currentAudio = null;
    playNextSpeech();
  };
  audio.play().catch(() => {
    state.currentAudio = null;
    playNextSpeech();
  });
}

async function askAssistant(text) {
  const started = performance.now();
  state.history.push({ role: "user", content: text });
  const assistantNode = addTurn("assistant", "");
  state.aborter = new AbortController();
  setStatus("Thinking");
  setRouteMode(state.routeMode, false);

  const response = await fetch("/api/live-turn", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      llm_endpoint: els.endpoint.value.trim(),
      fallback_endpoint: els.fallbackEndpoint.value.trim(),
      route_mode: state.routeMode,
      force_fallback: state.routeMode === "agent",
      model: els.model.value.trim() || "local-model",
      messages: state.history.slice(-12),
      voice: els.voice.value,
      temperature: requestOptions().temperature,
      top_p: requestOptions().topP,
      max_tokens: requestOptions().maxTokens,
    }),
    signal: state.aborter.signal,
  });

  if (!response.ok || !response.body) {
    throw new Error(`LLM request failed: ${response.status}`);
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let full = "";
  let pending = "";
  let firstTokenAt = null;

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    pending += decoder.decode(value, { stream: true });
    const lines = pending.split(/\r?\n/);
    pending = lines.pop() || "";

    for (const line of lines) {
      if (!line.trim()) continue;
      const event = JSON.parse(line);
      if (event.type === "text_delta") {
        if (!firstTokenAt) {
          firstTokenAt = performance.now();
          const llmMs = Math.round(firstTokenAt - started);
          const sttMs = state.lastSttStartedAt && state.lastSttEndedAt
            ? Math.round(state.lastSttEndedAt - state.lastSttStartedAt)
            : 0;
          const totalMs = state.lastUtteranceStartedAt
            ? Math.round(firstTokenAt - state.lastUtteranceStartedAt)
            : llmMs;
          els.latency.textContent = sttMs
            ? `Token: ${llmMs}ms · STT: ${sttMs}ms · Total: ${totalMs}ms`
            : `First token: ${llmMs}ms`;
        }
        full += event.delta;
        assistantNode.textContent = full;
        els.conversation.scrollTop = els.conversation.scrollHeight;
      } else if (event.type === "audio") {
        speakAudio(event.audio, event.mime);
      } else if (event.type === "tts_error") {
        setStatus(`TTS fallback: ${event.error}`);
      } else if (event.type === "error") {
        throw new Error(event.error);
      }
    }
  }

  state.history.push({ role: "assistant", content: full });
  state.history = [state.history[0], ...state.history.slice(-10)];
  state.aborter = null;
  state.turnCount += 1;
  els.turn.textContent = `Turns: ${state.turnCount}`;
}

function pauseRecognitionForAssistant() {
  if (!recognition || !state.recognizing) return;
  state.stoppingForAssistant = true;
  try {
    recognition.stop();
  } catch {}
}

function resumeRecognition() {
  if (!state.live || state.recognizing || !recognition) return;
  state.stoppingForAssistant = false;
  try {
    recognition.start();
    state.recognizing = true;
  } catch {}
}

function resetCurrentUtterance() {
  finalText = "";
  interimText = "";
  userNode = null;
  clearTimeout(silenceTimer);
  clearTimeout(maxRecordTimer);
}

async function commitUtterance() {
  const prompt = `${finalText} ${interimText}`.trim();
  resetCurrentUtterance();
  if (!prompt || !state.live) return;
  try {
    await askAssistant(prompt);
  } catch (error) {
    if (!isIntentionalAbort(error)) {
      addTurn("assistant", error.message);
      setStatus("Error");
    } else {
      setStatus(state.live ? "Listening" : "Idle");
    }
  }
}

function startRecognition() {
  if (!SpeechRecognition) {
    setStatus("SpeechRecognition is not available in this browser");
    return;
  }

  recognition = new SpeechRecognition();
  recognition.continuous = true;
  recognition.interimResults = true;
  recognition.lang = "en-US";

  recognition.onaudiostart = () => {
    els.meter.style.width = "54%";
  };

  recognition.onspeechstart = () => {
    els.meter.style.width = "100%";
    if (state.speaking || state.aborter) interrupt();
    setStatus("User speaking");
    resetCurrentUtterance();
    userNode = addTurn("user", "");
  };

  recognition.onresult = (event) => {
    interimText = "";
    for (let i = event.resultIndex; i < event.results.length; i++) {
      const transcript = event.results[i][0].transcript;
      if (event.results[i].isFinal) finalText += transcript;
      else interimText += transcript;
    }
    if (!userNode) userNode = addTurn("user", "");
    userNode.textContent = `${finalText} ${interimText}`.trim();

    clearTimeout(silenceTimer);
    silenceTimer = setTimeout(commitUtterance, settings().silenceMs);
  };

  recognition.onspeechend = () => {
    els.meter.style.width = "28%";
    setStatus("Transcribing");
    clearTimeout(silenceTimer);
    silenceTimer = setTimeout(commitUtterance, Math.max(220, settings().silenceMs - 200));
  };

  recognition.onerror = (event) => {
    if (event.error === "no-speech" || event.error === "aborted") return;
    setStatus(`Mic error: ${event.error}`);
  };

  recognition.onend = () => {
    state.recognizing = false;
    els.meter.style.width = "8%";
    if (state.live && !state.stoppingForAssistant && !state.speaking) {
      resumeRecognition();
    }
  };

  recognition.start();
  state.recognizing = true;
  setStatus("Listening");
}

function toggleLive() {
  state.live = !state.live;
  els.toggle.textContent = state.live ? "Stop live" : "Start live";
  els.talk.disabled = !(state.live && els.stt.value === "push");
  if (state.live) {
    if (els.stt.value === "local" || els.stt.value === "push") {
      startLocalStt();
    } else {
      startRecognition();
    }
  } else {
    interrupt();
    recognition?.stop();
    stopLocalStt();
    setStatus("Idle");
  }
}

els.toggle.addEventListener("click", toggleLive);
els.interrupt.addEventListener("click", interrupt);
els.stt.addEventListener("change", () => {
  els.talk.disabled = !(state.live && els.stt.value === "push");
  if (!state.live) return;
  recognition?.stop();
  stopLocalStt();
  if (els.stt.value === "local" || els.stt.value === "push") startLocalStt();
  else startRecognition();
});
els.talk.addEventListener("pointerdown", () => {
  if (state.live && els.stt.value === "push" && !state.recording) {
    interrupt();
    startLocalRecording();
  }
});
els.talk.addEventListener("pointerup", () => {
  if (state.live && els.stt.value === "push" && state.recording) {
    stopLocalRecording();
  }
});
els.talk.addEventListener("pointerleave", () => {
  if (state.live && els.stt.value === "push" && state.recording) {
    stopLocalRecording();
  }
});

async function startLocalStt() {
  try {
    state.mediaStream = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
      },
    });
    state.audioContext = new AudioContext();
    const source = state.audioContext.createMediaStreamSource(state.mediaStream);
    state.analyser = state.audioContext.createAnalyser();
    state.analyser.fftSize = 1024;
    source.connect(state.analyser);
    setStatus("Listening locally");
    state.micLevelTimer = setInterval(pollMicLevel, 80);
  } catch (error) {
    setStatus(`Local mic error: ${error.message}`);
  }
}

function stopLocalStt() {
  clearInterval(state.micLevelTimer);
  clearTimeout(state.silenceTimer);
  state.micLevelTimer = null;
  state.silenceTimer = null;
  if (state.mediaRecorder && state.mediaRecorder.state !== "inactive") {
    state.mediaRecorder.stop();
  }
  state.mediaStream?.getTracks().forEach((track) => track.stop());
  state.audioContext?.close().catch(() => {});
  state.mediaStream = null;
  state.audioContext = null;
  state.analyser = null;
  state.recording = false;
  state.localSpeechActive = false;
}

function pollMicLevel() {
  if (!state.analyser || !state.live || els.stt.value !== "local" || state.localTranscribing) return;
  if (performance.now() < vadCooldownUntil) return;
  const data = new Uint8Array(state.analyser.fftSize);
  state.analyser.getByteTimeDomainData(data);
  let sum = 0;
  for (const value of data) {
    const centered = (value - 128) / 128;
    sum += centered * centered;
  }
  const rms = Math.sqrt(sum / data.length);
  els.meter.style.width = `${Math.min(100, Math.max(8, rms * 420))}%`;

  if (rms >= settings().startThreshold) {
    if (state.speaking || state.aborter) interrupt();
    clearTimeout(state.silenceTimer);
    if (!state.recording) startLocalRecording();
    state.localSpeechActive = true;
    setStatus("User speaking");
  } else if (state.recording && state.localSpeechActive && rms <= settings().stopThreshold) {
    const elapsed = performance.now() - state.recordingStartedAt;
    if (elapsed < settings().minRecordMs) return;
    clearTimeout(state.silenceTimer);
    state.silenceTimer = setTimeout(stopLocalRecording, settings().silenceMs);
    setStatus("Transcribing");
  }
}

function startLocalRecording() {
  if (!state.mediaStream) return;
  state.recordingChunks = [];
  const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
    ? "audio/webm;codecs=opus"
    : "audio/webm";
  state.mediaRecorder = new MediaRecorder(state.mediaStream, { mimeType });
  userNode = addTurn("user", "");
  state.mediaRecorder.ondataavailable = (event) => {
    if (event.data.size > 0) state.recordingChunks.push(event.data);
  };
  state.mediaRecorder.onstop = submitLocalRecording;
  state.mediaRecorder.start();
  state.recording = true;
  state.recordingStartedAt = performance.now();
  state.lastUtteranceStartedAt = state.recordingStartedAt;
  clearTimeout(maxRecordTimer);
  maxRecordTimer = setTimeout(() => {
    if (state.recording) stopLocalRecording();
  }, els.stt.value === "push" ? 7000 : settings().maxRecordMs);
}

async function stopLocalRecording() {
  clearTimeout(state.silenceTimer);
  clearTimeout(maxRecordTimer);
  if (!state.mediaRecorder || state.mediaRecorder.state === "inactive") return;
  state.recording = false;
  state.localSpeechActive = false;
  vadCooldownUntil = performance.now() + 350;
  state.mediaRecorder.stop();
}

async function submitLocalRecording() {
  if (!state.recordingChunks.length || !state.live) return;
  const blob = new Blob(state.recordingChunks, { type: state.mediaRecorder.mimeType || "audio/webm" });
  if (blob.size < 1200) return;
  state.localTranscribing = true;
  state.lastSttStartedAt = performance.now();
  setStatus("Transcribing locally");
  try {
    const response = await fetch("/api/transcribe", {
      method: "POST",
      headers: {
        "Content-Type": blob.type,
        "X-STT-Model": els.sttModel.value,
        "X-STT-Language": els.sttLang.value,
      },
      body: blob,
    });
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || `STT failed: ${response.status}`);
    state.lastSttEndedAt = performance.now();
    const prompt = (result.text || "").trim();
    if (!prompt) {
      setStatus("Listening locally");
      return;
    }
    if (!userNode) userNode = addTurn("user", "");
    userNode.textContent = prompt;
    await askAssistant(prompt);
  } catch (error) {
    if (!isIntentionalAbort(error)) {
      addTurn("assistant", error.message);
      setStatus("Error");
    } else {
      setStatus(state.live ? "Listening locally" : "Idle");
    }
  } finally {
    state.localTranscribing = false;
  }
}

if (!SpeechRecognition && els.stt.value === "browser") {
  setStatus("Use Chrome or Edge for browser voice input");
}

loadSettings();
els.liveMode.addEventListener("click", () => setRouteMode("live"));
els.agentMode.addEventListener("click", () => setRouteMode("agent"));
for (const input of [
  els.mode,
  els.stt,
  els.sttModel,
  els.sttLang,
  els.endpoint,
  els.fallbackEndpoint,
  els.model,
  els.voice,
]) {
  input.addEventListener("change", saveSettings);
  input.addEventListener("input", saveSettings);
}
