import { useState, useRef, useCallback, useEffect } from 'react';
import { useAppStore, generateId } from '../../lib/store';
import { streamChat } from '../../lib/sse';
import { fetchSavings, getBase } from '../../lib/api';
import { MicButton } from './MicButton';
import { useSpeech } from '../../hooks/useSpeech';
import { JarvisVoiceOverlay } from './JarvisVoiceOverlay';
import type { ChatMessage, ToolCallInfo, TokenUsage, MessageTelemetry } from '../../types';

function toDisplayString(value: unknown): string {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function InputArea() {
  const [input, setInput] = useState('');
  const [showJarvis, setShowJarvis] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const abortRef = useRef<AbortController | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const activeId = useAppStore((s) => s.activeId);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const streamState = useAppStore((s) => s.streamState);
  const messages = useAppStore((s) => s.messages);
  const speechEnabled = useAppStore((s) => s.settings.speechEnabled);
  const maxTokens = useAppStore((s) => s.settings.maxTokens);
  const temperature = useAppStore((s) => s.settings.temperature);
  const createConversation = useAppStore((s) => s.createConversation);
  const addMessage = useAppStore((s) => s.addMessage);
  const updateLastAssistant = useAppStore((s) => s.updateLastAssistant);
  const setStreamState = useAppStore((s) => s.setStreamState);
  const resetStream = useAppStore((s) => s.resetStream);
  const modelLoading = useAppStore((s) => s.modelLoading);

  const { state: speechState, available: speechAvailable, startRecording, stopRecording } = useSpeech();

  const prevModelRef = useRef(selectedModel);
  useEffect(() => {
    if (prevModelRef.current !== selectedModel && streamState.isStreaming) {
      abortRef.current?.abort();
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
      resetStream();
      abortRef.current = null;
    }
    prevModelRef.current = selectedModel;
  }, [selectedModel, streamState.isStreaming, resetStream]);

  const micDisabled = !speechEnabled || !speechAvailable || streamState.isStreaming;
  const micReason: 'not-enabled' | 'no-backend' | 'streaming' | undefined =
    !speechEnabled ? 'not-enabled'
    : !speechAvailable ? 'no-backend'
    : streamState.isStreaming ? 'streaming'
    : undefined;



  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 200) + 'px';
  }, [input]);

  const stopStreaming = useCallback(() => {
    abortRef.current?.abort();
    if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
    resetStream();
  }, [resetStream]);

  const sendMessage = useCallback(async () => {
    const content = input.trim();
    if (!content || streamState.isStreaming) return;

    setInput('');

    let convId = activeId;
    if (!convId) convId = createConversation(selectedModel);

    const userMsg: ChatMessage = { id: generateId(), role: 'user', content, timestamp: Date.now() };
    addMessage(convId, userMsg);

    const currentMessages = useAppStore.getState().messages;
    const apiMessages = currentMessages.map((m) => ({ role: m.role, content: m.content }));

    const assistantMsg: ChatMessage = { id: generateId(), role: 'assistant', content: '', timestamp: Date.now() };
    addMessage(convId, assistantMsg);

    const startTime = Date.now();
    const timer = setInterval(() => { setStreamState({ elapsedMs: Date.now() - startTime }); }, 100);
    timerRef.current = timer;

    const controller = new AbortController();
    abortRef.current = controller;

    let accumulatedContent = '';
    let usage: TokenUsage | undefined;
    let complexity: { score: number; tier: string; suggested_max_tokens: number } | undefined;
    const toolCalls: ToolCallInfo[] = [];
    const skillStartedAt = new Map<string, number>();
    let lastFlush = 0;
    let ttftMs: number | undefined;

    setStreamState({ isStreaming: true, phase: 'Generating...', elapsedMs: 0, activeToolCalls: [], content: '' });
    useAppStore.getState().addLogEntry({
      timestamp: Date.now(), level: 'info', category: 'chat',
      message: `Request: "${content.slice(0, 80)}${content.length > 80 ? '...' : ''}" → ${selectedModel}`,
    });

    try {
      const agentToolsMarker = [
        {
          type: 'function',
          function: {
            name: 'sunday_agent_tools',
            description: 'Route this chat turn through the SUNDAY agent so configured tools and skills are available.',
            parameters: { type: 'object', properties: {} },
          },
        },
      ];

      for await (const sseEvent of streamChat(
        {
          model: selectedModel,
          messages: apiMessages,
          stream: true,
          temperature,
          max_tokens: maxTokens,
          tools: agentToolsMarker,
        },
        controller.signal,
      )) {
        const eventName = sseEvent.event;

        if (eventName === 'agent_turn_start') {
          setStreamState({ phase: 'Agent thinking...' });
        } else if (eventName === 'inference_start') {
          setStreamState({ phase: 'Generating...' });
        } else if (eventName === 'tool_call_start') {
          try {
            const data = JSON.parse(sseEvent.data);
            const tc: ToolCallInfo = {
              id: generateId(),
              tool: toDisplayString(data.tool || 'unknown-tool'),
              arguments: toDisplayString(data.arguments),
              status: 'running',
            };
            if (tc.tool.startsWith('skill_')) tc.kind = 'skill';
            toolCalls.push(tc);
            setStreamState({
              phase: `${tc.kind === 'skill' ? 'Using skill' : 'Calling'} ${tc.tool}...`,
              activeToolCalls: [...toolCalls],
            });
            updateLastAssistant(convId, accumulatedContent, [...toolCalls]);
          } catch {}
        } else if (eventName === 'tool_call_end') {
          try {
            const data = JSON.parse(sseEvent.data);
            const toolName = toDisplayString(data.tool || 'unknown-tool');
            const tc = toolCalls.find((t) => t.tool === toolName && t.status === 'running');
            if (tc) {
              tc.status = data.success ? 'success' : 'error';
              tc.latency = data.latency;
              tc.result = toDisplayString(data.result);
            }
            setStreamState({ phase: 'Generating...', activeToolCalls: [...toolCalls] });
            updateLastAssistant(convId, accumulatedContent, [...toolCalls]);
          } catch {}
        } else if (eventName === 'skill_execute_start') {
          try {
            const data = JSON.parse(sseEvent.data);
            const skillName = data.skill || data.name || 'unknown-skill';
            skillStartedAt.set(skillName, Date.now());
            const steps = typeof data.steps === 'number' ? `${data.steps} step${data.steps === 1 ? '' : 's'}` : '';
            const tc: ToolCallInfo = {
              id: generateId(),
              tool: skillName,
              kind: 'skill',
              arguments: steps,
              status: 'running',
            };
            toolCalls.push(tc);
            setStreamState({ phase: `Using skill ${skillName}...`, activeToolCalls: [...toolCalls] });
            updateLastAssistant(convId, accumulatedContent, [...toolCalls]);
          } catch {}
        } else if (eventName === 'skill_execute_end') {
          try {
            const data = JSON.parse(sseEvent.data);
            const skillName = data.skill || data.name || 'unknown-skill';
            const tc = [...toolCalls].reverse().find((t) => t.kind === 'skill' && t.tool === skillName && t.status === 'running');
            if (tc) {
              tc.status = data.success ? 'success' : 'error';
              const startedAt = skillStartedAt.get(skillName);
              if (startedAt) tc.latency = Date.now() - startedAt;
              if (data.result || data.output || data.error) {
                tc.result = String(data.result || data.output || data.error);
              }
            }
            setStreamState({ phase: 'Generating...', activeToolCalls: [...toolCalls] });
            updateLastAssistant(convId, accumulatedContent, [...toolCalls]);
          } catch {}
        } else {
          try {
            const data = JSON.parse(sseEvent.data);
            const delta = data.choices?.[0]?.delta;
            if (data.usage) usage = data.usage;
            if (data.complexity) complexity = data.complexity;
            if (delta?.content) {
              if (!ttftMs) ttftMs = Date.now() - startTime;
              accumulatedContent += delta.content;
              setStreamState({ content: accumulatedContent, phase: '' });
              const now = Date.now();
              if (now - lastFlush >= 80) {
                updateLastAssistant(convId, accumulatedContent, toolCalls.length > 0 ? [...toolCalls] : undefined);
                lastFlush = now;
              }
            }
            if (data.choices?.[0]?.finish_reason === 'stop') break;
          } catch {}
        }
      }
    } catch (err: any) {
      if (err.name === 'AbortError') {
        if (!accumulatedContent) accumulatedContent = '(Generation stopped)';
      } else {
        accumulatedContent = accumulatedContent || `Error: ${err?.message || String(err)}`;
      }
    } finally {
      if (!accumulatedContent) accumulatedContent = 'No response was generated. Please try again.';
      const totalMs = Date.now() - startTime;
      const _CLOUD_PREFIXES = ['gpt-', 'o1-', 'o3-', 'o4-', 'claude-', 'gemini-', 'openrouter/', 'MiniMax-', 'chatgpt-'];
      const engineLabel = _CLOUD_PREFIXES.some(p => selectedModel.startsWith(p)) ? 'cloud' : 'ollama';
      const telemetry: MessageTelemetry = {
        engine: engineLabel, model_id: selectedModel, total_ms: totalMs, ttft_ms: ttftMs,
        tokens_per_sec: usage?.completion_tokens ? usage.completion_tokens / (totalMs / 1000) : undefined,
        complexity_score: complexity?.score, complexity_tier: complexity?.tier, suggested_max_tokens: complexity?.suggested_max_tokens,
      };
      let audioMeta: { url: string } | undefined;
      try {
        const digestRes = await fetch(`${getBase()}/api/digest`);
        if (digestRes.ok) { const digest = await digestRes.json(); if (digest.audio_available) audioMeta = { url: `${getBase()}/api/digest/audio` }; }
      } catch {}
      updateLastAssistant(convId, accumulatedContent, toolCalls.length > 0 ? toolCalls : undefined, usage, telemetry, audioMeta);
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
      resetStream();
      abortRef.current = null;
      fetchSavings().then((data) => useAppStore.getState().setSavings(data)).catch(() => {});
    }
  }, [input, activeId, selectedModel, streamState.isStreaming, createConversation, addMessage, updateLastAssistant, setStreamState, resetStream]);

  const wasVoiceInputRef = useRef(false);

  const handleMicClick = useCallback(async () => {
    setShowJarvis(true);
  }, []);

  useEffect(() => {
    if (input && showJarvis === false && !streamState.isStreaming && wasVoiceInputRef.current) {
      wasVoiceInputRef.current = false;
      sendMessage();
    }
  }, [input, showJarvis, streamState.isStreaming, sendMessage]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const canSend = input.trim().length > 0 && !modelLoading;

  return (
    <div className="w-full shrink-0" style={{ background: 'transparent' }}>
      {showJarvis && (
        <JarvisVoiceOverlay 
          onTranscript={(userText, assistantText) => {
            // Add both to history directly
            const convId = activeId || createConversation(selectedModel);
            addMessage(convId, { id: generateId(), role: 'user', content: userText, timestamp: Date.now() });
            addMessage(convId, { id: generateId(), role: 'assistant', content: assistantText, timestamp: Date.now() });
            
            // Do NOT close automatically here, the HUD handles its own loop
          }}
          onClose={() => setShowJarvis(false)}
        />
      )}
      <div className="max-w-[var(--chat-max-width)] mx-auto px-4 pb-5 pt-2">
        <div className="relative">
          <div
            className="flex items-end gap-3 rounded-2xl px-5 py-4 transition-all duration-300"
            style={{
              background: 'rgba(30, 30, 30, 0.7)',
              backdropFilter: 'blur(20px) saturate(180%)',
              WebkitBackdropFilter: 'blur(20px) saturate(180%)',
              border: '1px solid rgba(255, 255, 255, 0.08)',
              boxShadow: '0 8px 32px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.05)',
            }}
          >
            <div className="flex-1 flex items-end gap-3">
              <MicButton
                state={speechState}
                onClick={handleMicClick}
                disabled={micDisabled}
                reason={micReason}
              />
              <textarea
                ref={textareaRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Ask me anything..."
                rows={1}
                className="flex-1 bg-transparent outline-none resize-none text-sm leading-relaxed placeholder:text-gray-500"
                style={{ 
                  color: '#e5e7eb', 
                  maxHeight: '200px',
                  fontSize: '15px',
                }}
                disabled={streamState.isStreaming || modelLoading}
              />
            </div>
            {streamState.isStreaming ? (
              <button
                onClick={stopStreaming}
                className="relative overflow-hidden group"
                style={{
                  width: '40px',
                  height: '40px',
                  borderRadius: '50%',
                  background: 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
                  cursor: 'pointer',
                  transition: 'all 0.3s ease',
                  boxShadow: '0 4px 12px rgba(239, 68, 68, 0.3)',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.transform = 'scale(1.05)';
                  e.currentTarget.style.boxShadow = '0 6px 16px rgba(239, 68, 68, 0.4)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.transform = 'scale(1)';
                  e.currentTarget.style.boxShadow = '0 4px 12px rgba(239, 68, 68, 0.3)';
                }}
                title="Stop generating"
              >
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
                  <div style={{ width: '12px', height: '12px', background: 'white', borderRadius: '2px' }} />
                </div>
              </button>
            ) : (
              <button
                onClick={sendMessage}
                disabled={!canSend}
                className="relative overflow-hidden"
                style={{
                  width: '40px',
                  height: '40px',
                  borderRadius: '50%',
                  background: canSend 
                    ? 'linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)'
                    : 'rgba(75, 85, 99, 0.5)',
                  cursor: canSend ? 'pointer' : 'default',
                  opacity: canSend ? 1 : 0.5,
                  transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                  boxShadow: canSend ? '0 4px 14px rgba(59, 130, 246, 0.4)' : 'none',
                }}
                onMouseEnter={(e) => { 
                  if (canSend) {
                    e.currentTarget.style.transform = 'scale(1.05)';
                    e.currentTarget.style.boxShadow = '0 6px 18px rgba(59, 130, 246, 0.5)';
                  }
                }}
                onMouseLeave={(e) => { 
                  if (canSend) {
                    e.currentTarget.style.transform = 'scale(1)';
                    e.currentTarget.style.boxShadow = '0 4px 14px rgba(59, 130, 246, 0.4)';
                  }
                }}
                title="Send message"
              >
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" style={{ color: canSend ? 'white' : '#6b7280' }}>
                    <path d="M22 2L11 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                    <path d="M22 2L15 22L11 13L2 9L22 2Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </div>
              </button>
            )}
          </div>
        </div>
        <p className="text-center text-[11px] mt-3" style={{ color: 'var(--color-text-tertiary)' }}>
          SUNDAY runs locally. Your data stays private.
        </p>
      </div>
    </div>
  );
}
