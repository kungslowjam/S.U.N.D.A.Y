import { useState, useEffect } from 'react';
import type { SpeechState } from '../../hooks/useSpeech';

interface MicButtonProps {
  state: SpeechState;
  onClick: () => void;
  disabled?: boolean;
  reason?: 'not-enabled' | 'no-backend' | 'streaming';
}

const pulseKeyframes = `
  @keyframes arc-reactor {
    0% { box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.7), 0 0 20px 5px rgba(59, 130, 246, 0.3); }
    70% { box-shadow: 0 0 0 15px rgba(59, 130, 246, 0), 0 0 25px 10px rgba(59, 130, 246, 0.5); }
    100% { box-shadow: 0 0 0 0 rgba(59, 130, 246, 0), 0 0 20px 5px rgba(59, 130, 246, 0.3); }
  }
  @keyframes arc-reactor-glow {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.8; transform: scale(0.95); }
  }
`;

export function MicButton({ state, onClick, disabled, reason }: MicButtonProps) {
  const [showTooltip, setShowTooltip] = useState(false);

  useEffect(() => {
    const styleId = 'arc-reactor-styles';
    if (!document.getElementById(styleId)) {
      const style = document.createElement('style');
      style.id = styleId;
      style.textContent = pulseKeyframes;
      document.head.appendChild(style);
    }
  }, []);

  const tooltipText =
    reason === 'not-enabled'
      ? 'Enable in Settings'
      : reason === 'no-backend'
        ? 'Speech backend not configured'
        : reason === 'streaming'
          ? 'Wait for response'
          : state === 'recording'
            ? 'Stop recording'
            : state === 'transcribing'
              ? 'Transcribing...'
              : 'Voice input';

  const isInactive = disabled || state === 'transcribing';
  const isRecording = state === 'recording';

  return (
    <div
      className="relative"
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
    >
      <button
        onClick={onClick}
        disabled={isInactive}
        className="relative group"
        style={{
          width: '44px',
          height: '44px',
          borderRadius: '50%',
          background: isRecording 
            ? 'radial-gradient(circle at center, #3b82f6 0%, #1d4ed8 70%, #1e3a8a 100%)'
            : isInactive
              ? 'rgba(107, 114, 128, 0.1)'
              : 'rgba(59, 130, 246, 0.1)',
          border: isRecording 
            ? '2px solid rgba(147, 197, 253, 0.8)'
            : '2px solid rgba(59, 130, 246, 0.3)',
          cursor: isInactive ? 'default' : 'pointer',
          opacity: isInactive ? 0.4 : 1,
          transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
          animation: isRecording ? 'arc-reactor 2s ease-in-out infinite' : 'none',
        }}
        onMouseEnter={(e) => {
          if (!isInactive && !isRecording) {
            e.currentTarget.style.background = 'rgba(59, 130, 246, 0.2)';
            e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.6)';
            e.currentTarget.style.boxShadow = '0 0 20px rgba(59, 130, 246, 0.4)';
          }
        }}
        onMouseLeave={(e) => {
          if (!isInactive && !isRecording) {
            e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)';
            e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.3)';
            e.currentTarget.style.boxShadow = 'none';
          }
        }}
      >
        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            animation: isRecording ? 'arc-reactor-glow 1s ease-in-out infinite' : 'none',
          }}
        >
          {state === 'transcribing' ? (
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" style={{ color: '#93c5fd' }}>
              <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2" strokeDasharray="40" strokeDashoffset="10">
                <animateTransform attributeName="transform" type="rotate" from="0 12 12" to="360 12 12" dur="1s" repeatCount="indefinite" />
              </circle>
            </svg>
          ) : isRecording ? (
            <div style={{ display: 'flex', gap: '3px', alignItems: 'center' }}>
              <div style={{ width: '4px', height: '4px', borderRadius: '50%', background: '#93c5fd', animation: 'arc-reactor-glow 0.5s ease-in-out infinite' }} />
              <div style={{ width: '4px', height: '4px', borderRadius: '50%', background: '#93c5fd', animation: 'arc-reactor-glow 0.5s ease-in-out infinite 0.15s' }} />
              <div style={{ width: '4px', height: '4px', borderRadius: '50%', background: '#93c5fd', animation: 'arc-reactor-glow 0.5s ease-in-out infinite 0.3s' }} />
            </div>
          ) : (
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" style={{ color: isInactive ? '#9ca3af' : '#60a5f7' }}>
              <path d="M12 14a3 3 0 0 1 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 1 3 3z" fill="currentColor" />
              <path d="M5 10a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v4a7 7 0 0 1-4 4v-2a1 1 0 0 0-1-1H6a1 1 0 0 1-1-1v-4z" fill="currentColor" opacity="0.7" />
              <path d="M19 10a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v4a7 7 0 0 0-4 4v-2a1 1 0 0 0-1-1h-1a1 1 0 0 1-1-1v-4z" fill="currentColor" opacity="0.7" />
              <path d="M12 20v2" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          )}
        </div>
      </button>
      {showTooltip && isInactive && (
        <div
          className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 rounded-full text-xs whitespace-nowrap pointer-events-none z-50"
          style={{
            background: 'rgba(30, 30, 30, 0.95)',
            color: '#f9fafb',
            boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            backdropFilter: 'blur(8px)',
            border: '1px solid rgba(255,255,255,0.1)',
          }}
        >
          {tooltipText}
        </div>
      )}
    </div>
  );
}
