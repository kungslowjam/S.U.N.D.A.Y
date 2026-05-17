import { useState, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import rehypeKatex from 'rehype-katex';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import 'katex/dist/katex.min.css';
import { Copy, Check, Volume2, Brain, ChevronDown, ChevronUp } from 'lucide-react';
import { AudioPlayer } from './AudioPlayer';
import { ToolCallCard } from './ToolCallCard';
import { XRayFooter } from './XRayFooter';
import type { ChatMessage } from '../../types';

interface ParsedContent {
  thinking: string;
  isThinkingActive: boolean;
  content: string;
}

function parseContent(text: string): ParsedContent {
  if (!text) return { thinking: '', isThinkingActive: false, content: '' };

  const thinkStart = text.indexOf('<think>');
  const thinkEnd = text.indexOf('</think>');

  if (thinkStart !== -1) {
    if (thinkEnd !== -1) {
      // Thinking completed
      const thinking = text.slice(thinkStart + 7, thinkEnd).trim();
      const content = text.slice(thinkEnd + 8).trim();
      return { thinking, isThinkingActive: false, content };
    } else {
      // Currently thinking (generating)
      const thinking = text.slice(thinkStart + 7).trim();
      return { thinking, isThinkingActive: true, content: '' };
    }
  }

  // Legacy fallback for "thinking" without tags (sometimes returned by older models)
  const legacyThinkStart = text.toLowerCase().indexOf('thinking\n');
  if (legacyThinkStart !== -1) {
    const nextText = text.slice(legacyThinkStart + 9);
    // Find double newline or end of thinking
    const legacyThinkEnd = nextText.indexOf('\n\n');
    if (legacyThinkEnd !== -1) {
      const thinking = nextText.slice(0, legacyThinkEnd).trim();
      const content = nextText.slice(legacyThinkEnd + 2).trim();
      return { thinking, isThinkingActive: false, content };
    }
  }

  return { thinking: '', isThinkingActive: false, content: text };
}

interface Props {
  message: ChatMessage;
}

function getTextContent(node: any): string {
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(getTextContent).join('');
  if (node?.props?.children) return getTextContent(node.props.children);
  return '';
}

function CodeBlockPre({ children, ...props }: any) {
  const [copied, setCopied] = useState(false);
  const codeElement = Array.isArray(children) ? children[0] : children;
  const className = codeElement?.props?.className || '';
  const match = /language-([\w-]+)/.exec(className);
  const lang = match ? match[1] : '';
  const code = getTextContent(codeElement?.props?.children).replace(/\n$/, '');

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="code-block-wrapper relative my-4" style={{ borderRadius: '12px', overflow: 'hidden', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
      <div
        className="flex items-center justify-between px-4 py-2 text-xs"
        style={{ background: 'rgba(30, 30, 30, 0.8)', color: '#9ca3af' }}
      >
        <span className="font-mono text-[11px]">{lang || 'code'}</span>
        <button
          onClick={handleCopy}
          className="flex items-center gap-1.5 px-2 py-1 rounded-md transition-colors cursor-pointer text-xs"
          style={{ color: '#9ca3af' }}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = '#e5e7eb';
            e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = '#9ca3af';
            e.currentTarget.style.background = 'transparent';
          }}
        >
          {copied ? <Check size={12} /> : <Copy size={12} />}
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre {...props} style={{ margin: 0, borderRadius: 0, background: 'rgba(20, 20, 20, 0.9)' }}>
        {children}
      </pre>
    </div>
  );
}

function CopyMessageButton({ content }: { content: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      onClick={handleCopy}
      className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 transition-all cursor-pointer"
      style={{ color: '#9ca3af' }}
      title="Copy message"
      onMouseEnter={(e) => {
        e.currentTarget.style.color = '#60a5f9';
        e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = '#9ca3af';
        e.currentTarget.style.background = 'transparent';
      }}
    >
      {copied ? <Check size={14} /> : <Copy size={14} />}
    </button>
  );
}

function SpeakMessageButton({ content }: { content: string }) {
  const supported = typeof window !== 'undefined' && 'speechSynthesis' in window;
  if (!supported || !content) return null;

  const handleSpeak = () => {
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(content);
    utterance.lang = navigator.language || 'th-TH';
    utterance.rate = 1;
    window.speechSynthesis.speak(utterance);
  };

  return (
    <button
      onClick={handleSpeak}
      className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 transition-all cursor-pointer"
      style={{ color: '#9ca3af' }}
      title="Speak message"
      onMouseEnter={(e) => {
        e.currentTarget.style.color = '#60a5f9';
        e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = '#9ca3af';
        e.currentTarget.style.background = 'transparent';
      }}
    >
      <Volume2 size={14} />
    </button>
  );
}

export function MessageBubble({ message }: Props) {
  const isUser = message.role === 'user';

  if (isUser) {
    return (
      <div className="flex justify-end mb-6">
        <div
          className="max-w-[85%] px-5 py-3.5 text-sm leading-relaxed"
          style={{
            background: 'linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)',
            color: 'white',
            borderRadius: '1.25rem',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            boxShadow: '0 4px 12px rgba(37, 99, 235, 0.3)',
          }}
        >
          {message.content}
        </div>
      </div>
    );
  }

  const [thinkingExpanded, setThinkingExpanded] = useState(false);

  const { thinking, isThinkingActive, content: cleanContent } = useMemo(
    () => parseContent(message.content),
    [message.content]
  );

  const isThinkingExpanded = thinkingExpanded || isThinkingActive;

  return (
    <div className="group mb-6">
      <div className="flex-1 min-w-0">
        {/* Thinking Block */}
        {(thinking || isThinkingActive) && (
          <div className="mb-3 rounded-xl border border-violet-500/10 bg-violet-950/10 backdrop-blur-sm overflow-hidden transition-all duration-300">
            <button
              onClick={() => setThinkingExpanded(!thinkingExpanded)}
              className="flex items-center justify-between w-full px-3 py-1.5 text-left text-[11px] font-medium text-violet-300 hover:text-violet-100 transition-all select-none cursor-pointer"
            >
              <div className="flex items-center gap-1.5">
                <Brain 
                  size={12} 
                  className={`text-violet-400 ${isThinkingActive ? 'animate-pulse' : ''}`} 
                />
                <span>
                  {isThinkingActive ? 'Thinking...' : 'Thought Process (Click to expand)'}
                </span>
              </div>
              <div className="text-violet-400">
                {isThinkingExpanded ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
              </div>
            </button>
            
            {isThinkingExpanded && (
              <div className="px-3 pb-2.5 pt-1 border-t border-violet-500/5 text-[11.5px] leading-relaxed text-slate-400 font-sans italic whitespace-pre-wrap select-text">
                {thinking}
                {isThinkingActive && (
                  <span className="inline-block w-1.5 h-3 ml-1 bg-violet-400 animate-pulse align-middle" />
                )}
              </div>
            )}
          </div>
        )}

        {message.toolCalls && message.toolCalls.length > 0 && (
          <div className="mb-4 flex flex-col gap-2">
            {message.toolCalls.map((tc) => (
              <ToolCallCard key={tc.id} toolCall={tc} />
            ))}
          </div>
        )}

        {message.audio?.url && <AudioPlayer src={message.audio.url} />}

        {cleanContent && (
          <div className="prose max-w-none" style={{ color: '#e5e7eb' }}>
            <ReactMarkdown
              remarkPlugins={[remarkGfm, remarkMath]}
              rehypePlugins={[[rehypeHighlight, { detect: true }], rehypeKatex]}
              components={{ pre: CodeBlockPre }}
            >
              {cleanContent}
            </ReactMarkdown>
          </div>
        )}

        <div className="flex items-center gap-2 mt-3">
          <SpeakMessageButton content={cleanContent} />
          <CopyMessageButton content={cleanContent} />
        </div>
        <XRayFooter usage={message.usage} telemetry={message.telemetry} />
      </div>
    </div>
  );
}
