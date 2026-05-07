import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ArrowUpRight,
  Check,
  Loader2,
  Plus,
  RefreshCw,
  Search,
  Sparkles,
  Trash2,
} from 'lucide-react';
import {
  fetchInstalledSkills,
  fetchSkillSources,
  fetchAvailableSkills,
  installSkill,
  removeSkill,
  syncSkills,
  type InstalledSkill,
  type SkillSource,
  type AvailableSkill,
} from '../lib/api';

const SOURCE_LABELS: Record<string, string> = {
  hermes: 'Hermes',
  openclaw: 'OpenClaw',
  officialskills: 'Official Skills',
  github: 'GitHub',
};

function sourceLabel(source: string) {
  return SOURCE_LABELS[source] || source;
}

function truncate(text = '', max = 170) {
  if (!text) return 'No description provided.';
  return text.length > max ? `${text.slice(0, max).trim()}...` : text;
}

function Badge({ children }: { children: React.ReactNode }) {
  return (
    <span
      className="inline-flex h-5 items-center rounded px-1.5 text-[11px]"
      style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-tertiary)' }}
    >
      {children}
    </span>
  );
}

function Empty({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="py-16 text-center">
      <div className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>
        {title}
      </div>
      <div className="mt-1 text-xs" style={{ color: 'var(--color-text-tertiary)' }}>
        {detail}
      </div>
    </div>
  );
}

export function SkillsPage() {
  const [installed, setInstalled] = useState<InstalledSkill[]>([]);
  const [sources, setSources] = useState<SkillSource[]>([]);
  const [available, setAvailable] = useState<AvailableSkill[]>([]);
  const [selectedSource, setSelectedSource] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [installedQuery, setInstalledQuery] = useState('');
  const [showAllInstalled, setShowAllInstalled] = useState(false);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const installedKeys = useMemo(
    () => new Set(installed.map((skill) => skill.name.toLowerCase())),
    [installed],
  );

  const visibleAvailable = useMemo(
    () => available.filter((skill) => !installedKeys.has(skill.name.toLowerCase())),
    [available, installedKeys],
  );

  const visibleInstalled = useMemo(() => {
    const query = installedQuery.trim().toLowerCase();
    if (!query) return installed;
    return installed.filter((skill) => {
      const haystack = [
        skill.name,
        skill.description || '',
        skill.category || '',
        ...(skill.tags || []),
      ].join(' ').toLowerCase();
      return haystack.includes(query);
    });
  }, [installed, installedQuery]);

  const displayedInstalled = useMemo(
    () => (showAllInstalled ? visibleInstalled : visibleInstalled.slice(0, 8)),
    [showAllInstalled, visibleInstalled],
  );

  const loadInstalled = useCallback(async () => {
    try {
      setInstalled(await fetchInstalledSkills());
    } catch {
      setInstalled([]);
    }
  }, []);

  const loadSources = useCallback(async () => {
    try {
      const nextSources = await fetchSkillSources();
      setSources(nextSources);
      if (nextSources.length > 0) {
        const defaultSource = (
          nextSources.find((source) => source.source === 'officialskills') || nextSources[0]
        ).source;
        setSelectedSource((current) => current || defaultSource);
      }
    } catch {
      setSources([]);
    }
  }, []);

  const loadAvailable = useCallback(async (showLoader = true) => {
    if (!selectedSource && !searchQuery) return;
    if (showLoader) setLoading(true);
    try {
      setAvailable(
        await fetchAvailableSkills(selectedSource || undefined, searchQuery || undefined),
      );
      setLastUpdated(new Date());
    } catch {
      setAvailable([]);
    } finally {
      if (showLoader) setLoading(false);
    }
  }, [searchQuery, selectedSource]);

  const refreshAll = useCallback(async (showLoader = false) => {
    setRefreshing(true);
    try {
      await Promise.all([
        loadInstalled(),
        loadAvailable(showLoader),
      ]);
      setLastUpdated(new Date());
    } finally {
      setRefreshing(false);
    }
  }, [loadAvailable, loadInstalled]);

  useEffect(() => {
    loadInstalled();
    loadSources();
  }, [loadInstalled, loadSources]);

  useEffect(() => {
    loadAvailable();
  }, [loadAvailable]);

  useEffect(() => {
    const refreshWhenActive = () => {
      if (document.visibilityState === 'visible') {
        refreshAll(false);
      }
    };
    const interval = window.setInterval(refreshWhenActive, 45_000);
    window.addEventListener('focus', refreshWhenActive);
    document.addEventListener('visibilitychange', refreshWhenActive);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', refreshWhenActive);
      document.removeEventListener('visibilitychange', refreshWhenActive);
    };
  }, [refreshAll]);

  const handleInstall = async (skill: AvailableSkill) => {
    const key = `${skill.source}:${skill.name}`;
    setInstalling(key);
    try {
      await installSkill(skill.source, skill.name, skill.url);
      await refreshAll(false);
    } catch (e: any) {
      alert(`Failed to install: ${e.message}`);
    } finally {
      setInstalling(null);
    }
  };

  const handleRemove = async (name: string) => {
    if (!confirm(`Remove skill "${name}"?`)) return;
    try {
      await removeSkill(name);
      await refreshAll(false);
    } catch (e: any) {
      alert(`Failed to remove: ${e.message}`);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      await syncSkills(selectedSource);
      await refreshAll(true);
    } catch (e: any) {
      alert(`Sync failed: ${e.message}`);
    } finally {
      setSyncing(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto px-6 py-8">
      <div className="mx-auto max-w-6xl">
        <header className="mb-6 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <div className="mb-2 flex items-center gap-2">
              <Sparkles size={18} style={{ color: 'var(--color-accent)' }} />
              <h1 className="text-xl font-semibold" style={{ color: 'var(--color-text)' }}>
                Skills
              </h1>
            </div>
            <p className="max-w-2xl text-sm" style={{ color: 'var(--color-text-secondary)' }}>
              Browse and install compact capability packs for SUNDAY agents.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <div
              className="flex h-9 items-center rounded-lg px-3 text-xs"
              style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-secondary)' }}
            >
              {installed.length} installed
            </div>
            <button
              onClick={() => refreshAll(false)}
              disabled={refreshing}
              className="flex h-9 items-center gap-2 rounded-lg px-3 text-sm font-medium disabled:opacity-60"
              style={{
                background: 'var(--color-surface)',
                color: 'var(--color-text)',
                border: '1px solid var(--color-border)',
              }}
            >
              {refreshing ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />}
              Refresh
            </button>
            <button
              onClick={handleSync}
              disabled={syncing}
              className="flex h-9 items-center gap-2 rounded-lg px-3 text-sm font-medium disabled:opacity-60"
              style={{
                background: 'var(--color-surface)',
                color: 'var(--color-text)',
                border: '1px solid var(--color-border)',
              }}
            >
              {syncing ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />}
              Sync
            </button>
          </div>
        </header>

        <section
          className="mb-5 grid gap-3 rounded-lg p-3 lg:grid-cols-[220px_1fr]"
          style={{ background: 'var(--color-surface)', border: '1px solid var(--color-border)' }}
        >
          <select
            value={selectedSource}
            onChange={(event) => setSelectedSource(event.target.value)}
            className="h-10 rounded-lg px-3 text-sm outline-none"
            style={{
              background: 'var(--color-bg-secondary)',
              color: 'var(--color-text)',
              border: '1px solid var(--color-border)',
            }}
          >
            <option value="">All sources</option>
            {sources.map((source) => (
              <option key={source.source} value={source.source}>
                {sourceLabel(source.source)}
              </option>
            ))}
          </select>

          <div className="relative">
            <Search
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2"
              style={{ color: 'var(--color-text-tertiary)' }}
            />
            <input
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="Search skills"
              className="h-10 w-full rounded-lg pl-9 pr-3 text-sm outline-none"
              style={{
                background: 'var(--color-bg-secondary)',
                color: 'var(--color-text)',
                border: '1px solid var(--color-border)',
              }}
            />
          </div>
        </section>

        <section
          className="mb-5 rounded-lg p-3"
          style={{ background: 'var(--color-surface)', border: '1px solid var(--color-border)' }}
        >
          <div className="mb-3 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div className="flex items-center gap-2">
              <div
                className="flex h-8 w-8 items-center justify-center rounded-lg"
                style={{ background: 'var(--color-bg-secondary)' }}
              >
                <Check size={15} style={{ color: 'var(--color-success)' }} />
              </div>
              <div>
                <h2 className="text-sm font-semibold" style={{ color: 'var(--color-text)' }}>
                  Installed
                </h2>
                <p className="text-xs" style={{ color: 'var(--color-text-tertiary)' }}>
                  {installed.length === 0
                    ? 'No active skills yet'
                    : `${installed.length} active skill${installed.length === 1 ? '' : 's'}`}
                </p>
              </div>
            </div>

            {installed.length > 0 && (
              <div className="relative w-full md:w-72">
                <Search
                  size={14}
                  className="absolute left-3 top-1/2 -translate-y-1/2"
                  style={{ color: 'var(--color-text-tertiary)' }}
                />
                <input
                  value={installedQuery}
                  onChange={(event) => {
                    setInstalledQuery(event.target.value);
                    setShowAllInstalled(false);
                  }}
                  placeholder="Find installed"
                  className="h-9 w-full rounded-lg pl-8 pr-3 text-xs outline-none"
                  style={{
                    background: 'var(--color-bg-secondary)',
                    color: 'var(--color-text)',
                    border: '1px solid var(--color-border)',
                  }}
                />
              </div>
            )}
          </div>

          {installed.length === 0 ? (
            <div
              className="rounded-lg px-3 py-4 text-xs"
              style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-tertiary)' }}
            >
              Install from the catalog below to make skills available to SUNDAY agents.
            </div>
          ) : visibleInstalled.length === 0 ? (
            <div
              className="rounded-lg px-3 py-4 text-xs"
              style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-tertiary)' }}
            >
              No installed skills match "{installedQuery}".
            </div>
          ) : (
            <>
              <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-4">
                {displayedInstalled.map((skill) => (
                  <div
                  key={skill.name}
                    className="group flex min-h-[58px] items-start justify-between gap-2 rounded-lg p-2.5"
                  style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-secondary)' }}
                  title={skill.name}
                >
                    <div className="min-w-0">
                      <div className="flex items-center gap-1.5">
                        <Check size={12} style={{ color: 'var(--color-success)' }} />
                        <span className="truncate text-xs font-medium" style={{ color: 'var(--color-text)' }}>
                          {skill.name}
                        </span>
                      </div>
                      <div className="mt-1 truncate text-[11px]" style={{ color: 'var(--color-text-tertiary)' }}>
                        {skill.category || skill.version || 'Installed skill'}
                      </div>
                    </div>
                  <button
                    onClick={() => handleRemove(skill.name)}
                      className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md opacity-70 transition-opacity hover:opacity-100"
                    style={{ color: 'var(--color-text-tertiary)' }}
                      title={`Remove ${skill.name}`}
                  >
                      <Trash2 size={13} />
                  </button>
                  </div>
                ))}
              </div>

              {visibleInstalled.length > 8 && (
                <button
                  onClick={() => setShowAllInstalled((value) => !value)}
                  className="mt-3 h-8 rounded-lg px-3 text-xs font-medium"
                  style={{
                    background: 'var(--color-bg-secondary)',
                    color: 'var(--color-text-secondary)',
                    border: '1px solid var(--color-border)',
                  }}
                >
                  {showAllInstalled
                    ? 'Show less'
                    : `Show ${visibleInstalled.length - 8} more installed`}
                </button>
              )}
            </>
          )}
        </section>

        <div>
          <section>
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-sm font-semibold" style={{ color: 'var(--color-text)' }}>
                Catalog
              </h2>
              <span className="text-xs" style={{ color: 'var(--color-text-tertiary)' }}>
                {loading ? 'Loading' : `${visibleAvailable.length} available`}
                {lastUpdated && !loading ? ` · updated ${lastUpdated.toLocaleTimeString()}` : ''}
              </span>
            </div>

            <div
              className="overflow-hidden rounded-lg"
              style={{ background: 'var(--color-surface)', border: '1px solid var(--color-border)' }}
            >
              {loading ? (
                <div className="flex items-center justify-center gap-2 py-16 text-sm" style={{ color: 'var(--color-text-secondary)' }}>
                  <Loader2 size={16} className="animate-spin" />
                  Loading catalog
                </div>
              ) : visibleAvailable.length === 0 ? (
                <Empty title="No catalog results" detail="Try a different source or search term." />
              ) : (
                <div className="max-h-[680px] divide-y overflow-y-auto" style={{ borderColor: 'var(--color-border)' }}>
                  {visibleAvailable.map((skill) => {
                    const key = `${skill.source}:${skill.name}`;
                    const busy = installing === key;
                    return (
                      <div key={key} className="p-4">
                        <div className="flex items-start justify-between gap-4">
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <h3 className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>
                                {skill.name}
                              </h3>
                              <Badge>{sourceLabel(skill.source)}</Badge>
                              {skill.catalog_only && <Badge>catalog</Badge>}
                            </div>
                            <p className="mt-2 text-xs leading-5" style={{ color: 'var(--color-text-secondary)' }}>
                              {truncate(skill.description, 210)}
                            </p>
                            <div className="mt-3 flex flex-wrap items-center gap-3 text-xs">
                              {skill.category && (
                                <span style={{ color: 'var(--color-text-tertiary)' }}>{skill.category}</span>
                              )}
                              {skill.url && (
                                <button
                                  onClick={() => window.open(skill.url, '_blank', 'noopener,noreferrer')}
                                  className="inline-flex items-center gap-1"
                                  style={{ color: 'var(--color-accent)' }}
                                >
                                  Source <ArrowUpRight size={12} />
                                </button>
                              )}
                            </div>
                          </div>
                          <button
                            onClick={() => handleInstall(skill)}
                            disabled={busy}
                            className="flex h-8 shrink-0 items-center gap-1.5 rounded-lg px-3 text-xs font-medium disabled:opacity-60"
                            style={{ background: 'var(--color-accent)', color: 'var(--color-on-accent)' }}
                          >
                            {busy ? <Loader2 size={13} className="animate-spin" /> : <Plus size={13} />}
                            {busy ? 'Installing' : 'Install'}
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
