import { useState, useEffect, useCallback } from 'react';
import { Database, MessageSquare, Brain, ArrowRight } from 'lucide-react';
import { useNavigate } from 'react-router';
import { listConnectors } from '../../lib/connectors-api';
import { fetchManagedAgents, fetchAgentChannels, getMemoryStats } from '../../lib/api';
import type { MemoryStats } from '../../lib/api';

export function KnowledgeChannelsOverview() {
  const navigate = useNavigate();
  const [sourceCount, setSourceCount] = useState(0);
  const [channelCount, setChannelCount] = useState(0);
  const [memoryChunks, setMemoryChunks] = useState(0);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [sources, agents, stats] = await Promise.all([
        listConnectors().catch(() => []),
        fetchManagedAgents().catch(() => []),
        getMemoryStats().catch(() => null as MemoryStats | null),
      ]);

      // Sources
      const connectedSources = sources.filter(s => s.connected).length;
      setSourceCount(connectedSources);

      // Channels (messaging channels bound to agents)
      let uniqueChannels = new Set<string>();
      await Promise.all(agents.map(async (agent: any) => {
        try {
          const channels = await fetchAgentChannels(agent.id);
          channels.forEach((c: any) => uniqueChannels.add(`${agent.id}:${c.channel_type}`));
        } catch { /* */ }
      }));
      setChannelCount(uniqueChannels.size);

      // Memory
      if (stats) {
        setMemoryChunks(stats.entries);
      }
    } catch {
      // 
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, [fetchData]);

  const items = [
    {
      label: 'Data Sources',
      icon: Database,
      value: sourceCount,
      detail: 'Connected sources',
      path: '/data-sources?tab=knowledge',
      color: 'var(--color-accent)',
    },
    {
      label: 'Messaging Channels',
      icon: MessageSquare,
      value: channelCount,
      detail: 'Active agent links',
      path: '/agents', // Messaging is per-agent for now
      color: 'var(--color-accent-purple)',
    },
    {
      label: 'Memory',
      icon: Brain,
      value: memoryChunks.toLocaleString(),
      detail: 'Indexed facts & history',
      path: '/data-sources?tab=memory',
      color: 'var(--color-success)',
    },
  ];

  return (
    <div className="hud-panel p-6">
      <h3 className="hud-label flex items-center gap-2 mb-4">
        <Sparkles size={12} style={{ color: 'var(--color-accent)' }} />
        Knowledge & Communication
      </h3>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {items.map((item) => (
          <div
            key={item.label}
            onClick={() => navigate(item.path)}
            className="group relative p-4 rounded-xl border border-transparent hover:border-[var(--color-border)] bg-[var(--color-bg-secondary)] transition-all cursor-pointer overflow-hidden"
          >
            <div className="absolute top-0 left-0 w-1 h-full" style={{ background: item.color }} />
            <div className="flex items-center justify-between mb-2">
              <item.icon size={16} style={{ color: item.color }} />
              <ArrowRight size={12} className="opacity-0 group-hover:opacity-50 transition-opacity" />
            </div>
            <div className="hud-mono text-2xl font-bold mb-1" style={{ color: 'var(--color-text)' }}>
              {item.value}
            </div>
            <div className="text-[10px] font-semibold uppercase tracking-wider mb-1" style={{ color: 'var(--color-text-secondary)' }}>
              {item.label}
            </div>
            <div className="text-[10px]" style={{ color: 'var(--color-text-tertiary)' }}>
              {item.detail}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Sparkles({ size, style }: { size: number; style?: any }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={style}>
      <path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z" />
      <path d="M5 3v4" />
      <path d="M19 17v4" />
      <path d="M3 5h4" />
      <path d="M17 19h4" />
    </svg>
  );
}
