import { useState } from 'react';
import { 
  ChevronDown, 
  ChevronRight, 
  Loader2, 
  CheckCircle2, 
  XCircle, 
  Sparkles, 
  Wrench,
  Globe, 
  Terminal, 
  FileText, 
  GitBranch, 
  Calculator, 
  Cpu, 
  Network, 
  Database,
  Eye,
  Settings,
  Copy,
  Check,
  Maximize2
} from 'lucide-react';
import type { ToolCallInfo } from '../../types';

interface Props {
  toolCall: ToolCallInfo;
}

const statusConfig = {
  running: { 
    icon: Loader2, 
    color: 'text-amber-400', 
    badgeBg: 'bg-amber-500/10 border-amber-500/20 text-amber-300',
    borderGlow: 'border-amber-500/30 shadow-[0_0_15px_-3px_rgba(245,158,11,0.15)] animate-pulse'
  },
  success: { 
    icon: CheckCircle2, 
    color: 'text-emerald-400', 
    badgeBg: 'bg-emerald-500/10 border-emerald-500/20 text-emerald-300',
    borderGlow: 'border-emerald-500/20 shadow-[0_0_15px_-3px_rgba(16,185,129,0.1)] hover:border-emerald-500/40'
  },
  error: { 
    icon: XCircle, 
    color: 'text-rose-400', 
    badgeBg: 'bg-rose-500/10 border-rose-500/20 text-rose-300',
    borderGlow: 'border-rose-500/30 shadow-[0_0_15px_-3px_rgba(244,63,94,0.15)]'
  },
};

const getToolIcon = (toolName: string) => {
  const name = toolName.toLowerCase();
  if (name.includes('search') || name.includes('find')) return Globe;
  if (name.includes('browser') || name.includes('click') || name.includes('type') || name.includes('navigate') || name.includes('extract')) return Eye;
  if (name.includes('shell') || name.includes('cmd') || name.includes('terminal') || name.includes('code') || name.includes('interpreter')) return Terminal;
  if (name.includes('file') || name.includes('read') || name.includes('write')) return FileText;
  if (name.includes('git')) return GitBranch;
  if (name.includes('calc') || name.includes('math')) return Calculator;
  if (name.includes('http') || name.includes('fetch') || name.includes('request')) return Network;
  if (name.includes('db') || name.includes('memory') || name.includes('store') || name.includes('sqlite')) return Database;
  if (name.includes('think')) return Cpu;
  return Settings;
};

function toDisplayString(value: unknown): string {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function previewArgs(value: unknown): string {
  const raw = toDisplayString(value);
  if (!raw) return '';
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object') {
      const entries = Object.entries(parsed);
      if (entries.length === 0) return '';
      const [k, v] = entries[0];
      const valStr =
        typeof v === 'string' ? v : JSON.stringify(v);
      const trimmed = valStr.length > 35 ? `${valStr.slice(0, 35)}…` : valStr;
      return entries.length === 1 ? `${k}: ${trimmed}` : `${k}: ${trimmed}, …`;
    }
  } catch {
    /* fall through */
  }
  return raw.length > 50 ? `${raw.slice(0, 50)}…` : raw;
}

export function ToolCallCard({ toolCall }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [copiedArgs, setCopiedArgs] = useState(false);
  const [copiedResult, setCopiedResult] = useState(false);

  const config = statusConfig[toolCall.status] || statusConfig.success;
  const StatusIcon = config.icon;
  const KindIcon = toolCall.kind === 'skill' ? Sparkles : Wrench;
  const ToolSpecificIcon = getToolIcon(toolCall.tool);
  const kindLabel = toolCall.kind === 'skill' ? 'Skill' : 'Tool';
  const preview = previewArgs(toolCall.arguments);

  const formattedArgs = toolCall.arguments ? formatJson(toolCall.arguments) : '';
  const formattedResult = toolCall.result ? toDisplayString(toolCall.result) : '';

  const copyToClipboard = (e: React.MouseEvent, text: string, type: 'args' | 'result') => {
    e.stopPropagation();
    navigator.clipboard.writeText(text);
    if (type === 'args') {
      setCopiedArgs(true);
      setTimeout(() => setCopiedArgs(false), 2000);
    } else {
      setCopiedResult(true);
      setTimeout(() => setCopiedResult(false), 2000);
    }
  };

  return (
    <div className="mb-1.5 select-text">
      <div
        className={`inline-flex items-center gap-2 rounded-full border transition-all duration-300 ease-in-out ${config.borderGlow} bg-slate-950/40 backdrop-blur-sm border-slate-800/60 px-2.5 py-0.5 max-w-full`}
      >
        {/* Status Indicator */}
        <div className="flex-shrink-0 flex items-center">
          <StatusIcon
            size={9.5}
            className={`${config.color} ${toolCall.status === 'running' ? 'animate-spin' : ''}`}
          />
        </div>

        {/* Tool Name with Specific Icon */}
        <div className="flex items-center gap-1 font-sans font-medium text-slate-350 text-[10.5px] flex-shrink-0">
          <ToolSpecificIcon size={10} className="text-slate-500" />
          <span>{toolCall.tool}</span>
        </div>

        {/* Arguments Preview */}
        {preview && (
          <span
            className="truncate text-[9px] text-slate-500 font-mono pl-1.5 border-l border-slate-800/60 max-w-[160px] sm:max-w-[240px] select-text"
          >
            {preview}
          </span>
        )}

        {/* Latency badge */}
        {toolCall.latency != null && (
          <span
            className="text-[8.5px] font-mono text-slate-500 flex-shrink-0 ml-0.5"
          >
            ({toolCall.latency < 1000
              ? `${Math.round(toolCall.latency)}ms`
              : `${(toolCall.latency / 1000).toFixed(2)}s`})
          </span>
        )}

        {/* Small expand action icon */}
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-slate-500 hover:text-slate-350 transition-colors p-0.5 rounded cursor-pointer ml-0.5 flex items-center select-none"
        >
          {expanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        </button>
      </div>

      {/* Accordion Expansion (Indented Tree Branch) */}
      {expanded && (
        <div className="mt-1 pl-4 max-w-xl text-[10px] font-mono text-slate-400 space-y-1.5 border-l border-slate-800 ml-3.5 select-text">
          {/* Arguments block */}
          {!!formattedArgs && (
            <div className="bg-slate-950/20 p-1.5 rounded border border-slate-900 select-text">
              <div className="text-[8.5px] text-slate-500 font-sans font-semibold tracking-wider uppercase mb-0.5 flex items-center justify-between select-none">
                <span>Arguments</span>
                <button
                  onClick={(e) => copyToClipboard(e, formattedArgs, 'args')}
                  className="hover:text-slate-300 transition-colors cursor-pointer"
                >
                  {copiedArgs ? 'Copied' : 'Copy'}
                </button>
              </div>
              <pre className="whitespace-pre-wrap break-all leading-tight max-h-[60px] overflow-auto custom-scrollbar scrollbar-thin text-slate-400 select-text">
                {formattedArgs}
              </pre>
            </div>
          )}

          {/* Result block */}
          {!!formattedResult && (
            <div className="bg-slate-950/30 p-1.5 rounded border border-slate-900 select-text">
              <div className="text-[8.5px] text-slate-500 font-sans font-semibold tracking-wider uppercase mb-0.5 flex items-center justify-between select-none">
                <span>Result</span>
                <button
                  onClick={(e) => copyToClipboard(e, formattedResult, 'result')}
                  className="hover:text-slate-300 transition-colors cursor-pointer"
                >
                  {copiedResult ? 'Copied' : 'Copy'}
                </button>
              </div>
              <pre className="whitespace-pre-wrap break-all leading-tight max-h-[90px] overflow-auto custom-scrollbar scrollbar-thin text-slate-400 select-text">
                {formattedResult}
              </pre>
            </div>
          )}

          {/* Vision screen snippet */}
          {(!!toolCall.metadata?.screenshot_base64 || !!toolCall.metadata?.screenshot_path) && (
            <div className="mt-2 bg-slate-950/30 p-1.5 rounded border border-slate-900">
              <div className="text-[8.5px] text-slate-500 font-sans font-semibold tracking-wider uppercase mb-1">
                Vision Screenshot
              </div>
              <div className="rounded overflow-hidden">
                <img 
                  src={
                    toolCall.metadata.screenshot_base64
                      ? `data:image/jpeg;base64,${toolCall.metadata.screenshot_base64}`
                      : toolCall.metadata.screenshot_path?.startsWith('file:///')
                      ? `sunday-img://localhost/${toolCall.metadata.screenshot_path.slice(8)}`
                      : toolCall.metadata.screenshot_path?.startsWith('file://')
                      ? `sunday-img://localhost/${toolCall.metadata.screenshot_path.slice(7)}`
                      : toolCall.metadata.screenshot_path
                  } 
                  alt="Vision" 
                  className="w-full rounded cursor-zoom-in"
                  style={{ objectFit: 'contain', maxHeight: '180px', display: 'block' }}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function formatJson(raw: unknown): string {
  const text = toDisplayString(raw);
  try {
    const parsed = JSON.parse(text);
    if (parsed && typeof parsed === 'object') {
      delete (parsed as any).thought;
    }
    return JSON.stringify(parsed, null, 2);
  } catch {
    return text;
  }
}
