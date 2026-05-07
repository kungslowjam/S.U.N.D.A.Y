import { useRef, useEffect, useState, useCallback } from 'react';
import { useNavigate } from 'react-router';
import { MessageBubble } from './MessageBubble';
import { InputArea } from './InputArea';
import { StreamingDots } from './StreamingDots';
import { useAppStore } from '../../lib/store';
import { PanelRightOpen, PanelRightClose, Database, Sparkles } from 'lucide-react';
import { listConnectors } from '../../lib/connectors-api';

const spinKeyframes = `
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
`;

function getGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return 'Good morning';
  if (hour < 18) return 'Good afternoon';
  return 'Good evening';
}

const SUGGESTIONS = [
  'Explain quantum computing in simple terms',
  'Write a thank-you email to a colleague',
  'Help me debug a Python error',
  'Summarize a research paper for me',
];

export function ChatArea() {
  const messages = useAppStore((s) => s.messages);
  const streamState = useAppStore((s) => s.streamState);
  const systemPanelOpen = useAppStore((s) => s.systemPanelOpen);
  const toggleSystemPanel = useAppStore((s) => s.toggleSystemPanel);
  const navigate = useNavigate();
  const listRef = useRef<HTMLDivElement>(null);
  const shouldAutoScroll = useRef(true);

  const [hasConnectedSources, setHasConnectedSources] = useState<boolean | null>(null);

  useEffect(() => {
    const styleId = 'spin-animation-style';
    if (!document.getElementById(styleId)) {
      const style = document.createElement('style');
      style.id = styleId;
      style.textContent = spinKeyframes;
      document.head.appendChild(style);
    }
  }, []);

  useEffect(() => {
    listConnectors()
      .then((list) => setHasConnectedSources(list.some((c) => c.connected)))
      .catch(() => setHasConnectedSources(null));
  }, []);

  useEffect(() => {
    if (shouldAutoScroll.current && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages, streamState.content]);

  const handleScroll = () => {
    if (!listRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = listRef.current;
    shouldAutoScroll.current = scrollHeight - scrollTop - clientHeight < 100;
  };

  const handleSuggestionClick = useCallback((suggestion: string) => {
    const textarea = document.querySelector('textarea') as HTMLTextAreaElement | null;
    if (textarea) {
      const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
        window.HTMLTextAreaElement.prototype, 'value'
      )?.set;
      nativeInputValueSetter?.call(textarea, suggestion);
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      textarea.focus();
    }
  }, []);

  const isEmpty = messages.length === 0 && !streamState.isStreaming;

  return (
    <div className="flex flex-col h-full" style={{ background: 'transparent' }}>
      <div className="flex items-center justify-end px-4 py-2 shrink-0">
        <button
          onClick={toggleSystemPanel}
          className="p-1.5 rounded-lg transition-colors cursor-pointer"
          style={{ color: 'var(--color-text-tertiary)' }}
          title={`${systemPanelOpen ? 'Hide' : 'Show'} system panel`}
          onMouseEnter={(e) => (e.currentTarget.style.color = 'var(--color-text)')}
          onMouseLeave={(e) => (e.currentTarget.style.color = 'var(--color-text-tertiary)')}
        >
          {systemPanelOpen ? <PanelRightClose size={16} /> : <PanelRightOpen size={16} />}
        </button>
      </div>

      <div ref={listRef} onScroll={handleScroll} className="flex-1 overflow-y-auto">
        {isEmpty ? (
          <div className="flex flex-col items-center justify-center h-full px-4" style={{ marginTop: '-4%' }}>
            <div
              className="relative mb-8"
              style={{
                width: '100px',
                height: '100px',
                borderRadius: '50%',
                background: 'linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(37, 99, 235, 0.1) 100%)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                boxShadow: '0 0 60px rgba(59, 130, 246, 0.3), 0 0 100px rgba(59, 130, 246, 0.1)',
              }}
            >
              <div
                style={{
                  position: 'absolute',
                  inset: '-2px',
                  borderRadius: '50%',
                  border: '2px solid transparent',
                  background: 'linear-gradient(135deg, rgba(59, 130, 246, 0.6), transparent) border-box',
                  WebkitMask: 'linear-gradient(#fff 0 0) padding-box, linear-gradient(#fff 0 0)',
                  WebkitMaskComposite: 'xor',
                  maskComposite: 'exclude',
                  animation: 'spin 8s linear infinite',
                }}
              />
              <Sparkles size={40} style={{ color: '#60a5f9' }} />
            </div>

            <h1 className="text-3xl font-light mb-2" style={{ color: '#e5e7eb' }}>
              {getGreeting()}
            </h1>
            <p className="text-sm mb-10" style={{ color: '#9ca3af' }}>
              How can I help you today?
            </p>

            <div className="grid grid-cols-2 gap-3 max-w-2xl w-full mb-8">
              {SUGGESTIONS.map((suggestion, i) => (
                <button
                  key={i}
                  onClick={() => handleSuggestionClick(suggestion)}
                  className="text-left px-5 py-4 rounded-xl text-sm transition-all cursor-pointer"
                  style={{
                    background: 'rgba(30, 30, 30, 0.6)',
                    border: '1px solid rgba(255, 255, 255, 0.06)',
                    color: '#d1d5db',
                    backdropFilter: 'blur(10px)',
                    transition: 'all 0.3s ease',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'rgba(59, 130, 246, 0.15)';
                    e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.4)';
                    e.currentTarget.style.transform = 'translateY(-2px)';
                    e.currentTarget.style.boxShadow = '0 8px 25px rgba(59, 130, 246, 0.2)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'rgba(30, 30, 30, 0.6)';
                    e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.06)';
                    e.currentTarget.style.transform = 'translateY(0)';
                    e.currentTarget.style.boxShadow = 'none';
                  }}
                >
                  {suggestion}
                </button>
              ))}
            </div>

            {hasConnectedSources === false && (
              <button
                onClick={() => navigate('/data-sources')}
                className="flex items-center gap-2 px-5 py-3 rounded-xl text-xs cursor-pointer transition-colors"
                style={{
                  background: 'rgba(30, 30, 30, 0.6)',
                  border: '1px solid rgba(255, 255, 255, 0.06)',
                  color: '#9ca3af',
                  backdropFilter: 'blur(10px)',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)';
                  e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.3)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'rgba(30, 30, 30, 0.6)';
                  e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.06)';
                }}
              >
                <Database size={14} />
                Connect your data for personalized answers
              </button>
            )}
          </div>
        ) : (
          <div className="max-w-[var(--chat-max-width)] mx-auto px-4 py-6">
            {messages.map((msg) => (
              <MessageBubble key={msg.id} message={msg} />
            ))}
            {streamState.isStreaming && streamState.content === '' && (
              <div className="flex justify-start mb-4">
                <StreamingDots phase={streamState.phase} />
              </div>
            )}
          </div>
        )}
      </div>
      <InputArea />
    </div>
  );
}
