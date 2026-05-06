import { useState, useEffect } from 'react';
import {
  Plus,
  RefreshCw,
  X,
  Sparkles,
  Search,
  Trash2,
  Info,
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

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div
      className="rounded-xl p-5"
      style={{ background: 'var(--color-surface)', border: '1px solid var(--color-border)' }}
    >
      <h3 className="text-sm font-semibold mb-4" style={{ color: 'var(--color-text)' }}>
        {title}
      </h3>
      {children}
    </div>
  );
}

export function SkillsPage() {
  const [installed, setInstalled] = useState<InstalledSkill[]>([]);
  const [sources, setSources] = useState<SkillSource[]>([]);
  const [available, setAvailable] = useState<AvailableSkill[]>([]);
  const [showInstall, setShowInstall] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [selectedSource, setSelectedSource] = useState('');
  const [showInfo, setShowInfo] = useState<string | null>(null);

  const loadInstalled = async () => {
    try {
      const skills = await fetchInstalledSkills();
      setInstalled(skills);
    } catch {}
  };

  const loadSources = async () => {
    try {
      const srcs = await fetchSkillSources();
      setSources(srcs);
      if (srcs.length > 0 && !selectedSource) {
        setSelectedSource(srcs[0].source);
      }
    } catch {}
  };

  const loadAvailable = async () => {
    if (!selectedSource && !searchQuery) return;
    setLoading(true);
    try {
      const skills = await fetchAvailableSkills(selectedSource || undefined, searchQuery || undefined);
      setAvailable(skills);
    } catch {}
    setLoading(false);
  };

  useEffect(() => {
    loadInstalled();
    loadSources();
  }, []);

  useEffect(() => {
    if (showInstall) {
      loadAvailable();
    }
  }, [showInstall, selectedSource, searchQuery]);

  const handleInstall = async (skill: AvailableSkill) => {
    if (skill.catalog_only) {
      if (skill.url) window.open(skill.url, '_blank', 'noopener,noreferrer');
      return;
    }
    setInstalling(skill.name);
    try {
      await installSkill(skill.source, skill.name);
      await loadInstalled();
      setAvailable(available.filter(s => s.name !== skill.name));
    } catch (e: any) {
      alert(`Failed to install: ${e.message}`);
    }
    setInstalling(null);
  };

  const handleRemove = async (name: string) => {
    if (!confirm(`Remove skill "${name}"?`)) return;
    try {
      await removeSkill(name);
      await loadInstalled();
    } catch (e: any) {
      alert(`Failed to remove: ${e.message}`);
    }
  };

  const handleSync = async () => {
    setLoading(true);
    try {
      await syncSkills(selectedSource);
      await loadInstalled();
    } catch (e: any) {
      alert(`Sync failed: ${e.message}`);
    }
    setLoading(false);
  };

  return (
    <div className="flex-1 overflow-y-auto px-6 py-10">
      <div className="max-w-3xl mx-auto">
        <header className="mb-6">
          <h1 className="text-lg font-semibold" style={{ color: 'var(--color-text)' }}>
            Skills
          </h1>
          <p className="text-sm mt-2 max-w-2xl" style={{ color: 'var(--color-text-secondary)' }}>
            Install and manage skills from Hermes (~150), OpenClaw (~13,700), or any GitHub repo.
          </p>
        </header>

        <div className="flex flex-col gap-4">
          {/* Installed Skills */}
          <Section title="Installed Skills">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm" style={{ color: 'var(--color-text-secondary)' }}>
                {installed.length} skill(s) installed
              </span>
              <div className="flex gap-2">
                <button
                  onClick={() => setShowInstall(!showInstall)}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
                  style={{ background: 'var(--color-accent)', color: 'white' }}
                >
                  <Plus size={14} /> Install
                </button>
                <button
                  onClick={handleSync}
                  disabled={loading}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
                  style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text-secondary)', border: '1px solid var(--color-border)' }}
                >
                  <RefreshCw size={14} className={loading ? 'animate-spin' : ''} /> Sync All
                </button>
              </div>
            </div>

            {installed.length === 0 ? (
              <div className="text-sm py-4" style={{ color: 'var(--color-text-tertiary)' }}>
                No skills installed. Click "Install" to browse available skills.
              </div>
            ) : (
              <div className="grid gap-2">
                {installed.map(skill => (
                  <div
                    key={skill.name}
                    className="flex items-center justify-between p-3 rounded-lg"
                    style={{ background: 'var(--color-bg-secondary)' }}
                  >
                    <div className="flex items-center gap-3">
                      <Sparkles size={18} style={{ color: 'var(--color-accent)' }} />
                      <div>
                        <div className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>
                          {skill.name}
                          {skill.version && <span className="text-xs ml-1" style={{ color: 'var(--color-text-tertiary)' }}>v{skill.version}</span>}
                        </div>
                        {skill.description && (
                          <div className="text-xs mt-0.5" style={{ color: 'var(--color-text-tertiary)' }}>
                            {skill.description.slice(0, 80)}...
                          </div>
                        )}
                        {skill.tags && skill.tags.length > 0 && (
                          <div className="flex gap-1 mt-1">
                            {skill.tags.slice(0, 3).map(tag => (
                              <span key={tag} className="px-1.5 py-0.5 rounded text-[10px]" style={{ background: 'var(--color-bg-tertiary)', color: 'var(--color-text-tertiary)' }}>
                                {tag}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => setShowInfo(showInfo === skill.name ? null : skill.name)}
                        className="p-1.5 rounded hover:bg-[var(--color-bg-tertiary)] cursor-pointer"
                        style={{ color: 'var(--color-text-tertiary)' }}
                      >
                        <Info size={14} />
                      </button>
                      <button
                        onClick={() => handleRemove(skill.name)}
                        className="p-1.5 rounded hover:bg-red-500/20 cursor-pointer"
                        style={{ color: 'var(--color-error)' }}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Section>

          {/* Install Panel */}
          {showInstall && (
            <Section title="Browse & Install">
              <div className="flex gap-3 mb-4">
                <select
                  value={selectedSource}
                  onChange={e => setSelectedSource(e.target.value)}
                  className="text-sm px-3 py-2 rounded-lg outline-none cursor-pointer"
                  style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text)', border: '1px solid var(--color-border)' }}
                >
                  <option value="">All sources</option>
                  {sources.map(s => (
                    <option key={s.source} value={s.source}>{s.source}</option>
                  ))}
                </select>
                <div className="flex-1 relative">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--color-text-tertiary)' }} />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={e => setSearchQuery(e.target.value)}
                    placeholder="Search skills by name or description..."
                    className="w-full pl-9 pr-3 py-2 rounded-lg text-sm outline-none"
                    style={{ background: 'var(--color-bg-secondary)', color: 'var(--color-text)', border: '1px solid var(--color-border)' }}
                  />
                </div>
              </div>

              {loading ? (
                <div className="text-sm py-4" style={{ color: 'var(--color-text-tertiary)' }}>Loading...</div>
              ) : available.length === 0 ? (
                <div className="text-sm py-4" style={{ color: 'var(--color-text-tertiary)' }}>
                  {searchQuery ? 'No skills found' : 'Configure skill sources in config.toml to browse available skills'}
                </div>
              ) : (
                <div className="max-h-96 overflow-y-auto space-y-2">
                  {available.map(skill => (
                    <div
                      key={`${skill.source}:${skill.name}`}
                      className="flex items-center justify-between p-3 rounded-lg"
                      style={{ background: 'var(--color-bg-secondary)' }}
                    >
                      <div>
                        <div className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>
                          {skill.name}
                        </div>
                        <div className="text-xs mt-0.5" style={{ color: 'var(--color-text-tertiary)' }}>
                          <span className="px-1.5 py-0.5 rounded" style={{ background: 'var(--color-accent)', color: 'white' }}>{skill.source}</span>
                          {' / '}{skill.category}
                          {skill.catalog_only && ' / catalog'}
                          {skill.description && ` — ${skill.description}`}
                        </div>
                      </div>
                      <button
                        onClick={() => handleInstall(skill)}
                        disabled={installing === skill.name}
                        className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
                        style={{ background: 'var(--color-accent)', color: 'white' }}
                      >
                        {skill.catalog_only ? 'Open' : installing === skill.name ? 'Installing...' : <><Plus size={12} /> Install</>}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </Section>
          )}

          {/* Help */}
          <Section title="About Skills">
            <div className="text-sm space-y-2" style={{ color: 'var(--color-text-secondary)' }}>
              <p>Skills teach agents how to use tools and improve their reasoning.</p>
              <p>Install from:</p>
              <ul className="list-disc list-inside text-xs space-y-1" style={{ color: 'var(--color-text-tertiary)' }}>
                <li><strong>hermes</strong> - NousResearch/hermes-agent (~150 skills)</li>
                <li><strong>openclaw</strong> - OpenClaw community (~13,700 skills)</li>
                <li><strong>github</strong> - Any GitHub repo with SKILL.md</li>
              </ul>
              <p className="text-xs mt-2" style={{ color: 'var(--color-text-tertiary)' }}>
                Configure sources in <code>config.toml</code> under <code>[skills.sources]</code>
              </p>
            </div>
          </Section>
        </div>
      </div>
    </div>
  );
}
