import { useNavigate, useLocation } from 'react-router';
import {
  Plus,
  PanelLeftClose,
  PanelLeft,
  Cpu,
  Moon,
  Sun,
  Loader2,
  Settings,
  BarChart3,
  Bot,
  Database,
  ScrollText,
  Rocket,
  ChevronDown,
  Sparkles,
} from 'lucide-react';
import { ConversationList } from './ConversationList';
import { useAppStore } from '../../lib/store';

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();

  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const createConversation = useAppStore((s) => s.createConversation);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);
  const modelLoading = useAppStore((s) => s.modelLoading);
  const settings = useAppStore((s) => s.settings);
  const updateSettings = useAppStore((s) => s.updateSettings);
  const messages = useAppStore((s) => s.messages);

  const handleNewChat = () => {
    if (messages.length === 0) {
      navigate('/');
      return;
    }
    createConversation(selectedModel);
    navigate('/');
  };

  const navItems = [
    { path: '/dashboard', icon: BarChart3, label: 'Dashboard' },
    { path: '/data-sources', icon: Database, label: 'Data Sources' },
    { path: '/agents', icon: Bot, label: 'Agents' },
    { path: '/skills', icon: Sparkles, label: 'Skills' },
    { path: '/logs', icon: ScrollText, label: 'Logs' },
    { path: '/settings', icon: Settings, label: 'Settings' },
    { path: '/get-started', icon: Rocket, label: 'Get Started' },
  ];

  return (
    <>
      {!sidebarOpen && (
        <button
          onClick={toggleSidebar}
          className="fixed top-3 left-3 z-30 p-2 rounded-lg transition-colors cursor-pointer"
          style={{ color: 'var(--color-text-secondary)', background: 'var(--color-bg-secondary)' }}
          onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-tertiary)')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
        >
          <PanelLeft size={18} />
        </button>
      )}

      <aside
        className={`
          flex flex-col h-full shrink-0 transition-all duration-200 ease-in-out overflow-hidden
          fixed md:relative z-30
          ${sidebarOpen ? 'w-[260px]' : 'w-0'}
        `}
        style={{
          background: 'var(--color-sidebar)',
          borderRight: sidebarOpen ? '1px solid var(--color-border)' : 'none',
        }}
      >
        <div className="flex flex-col h-full w-[260px]">
          {/* Header: toggle + new chat */}
          <div className="flex items-center justify-between px-3 pt-3 pb-2">
            <button
              onClick={toggleSidebar}
              className="p-2 rounded-lg transition-colors cursor-pointer"
              style={{ color: 'var(--color-text-secondary)' }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <PanelLeftClose size={18} />
            </button>
            <button
              onClick={handleNewChat}
              className="p-2 rounded-lg transition-colors cursor-pointer"
              style={{ color: 'var(--color-text-secondary)' }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              title="New chat"
            >
              <Plus size={18} />
            </button>
          </div>

          {/* New Chat button */}
          <div className="px-3 mb-3">
            <button
              onClick={handleNewChat}
              className="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm transition-colors cursor-pointer"
              style={{
                background: 'var(--color-bg-secondary)',
                color: 'var(--color-text)',
                border: '1px solid var(--color-border)',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-tertiary)')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
            >
              <Plus size={16} />
              New chat
            </button>
          </div>

          {/* Model selector */}
          <div className="px-3 mb-3">
            <button
              onClick={() => setCommandPaletteOpen(true)}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-xl text-xs transition-colors cursor-pointer"
              style={{
                background: 'var(--color-bg-secondary)',
                color: 'var(--color-text-secondary)',
                border: '1px solid var(--color-border)',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-tertiary)')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
            >
              {modelLoading ? (
                <Loader2 size={14} className="animate-spin" style={{ color: 'var(--color-text-secondary)' }} />
              ) : (
                <Cpu size={14} />
              )}
              <div className="flex-1 min-w-0 text-left">
                <span className="truncate block" style={{ color: 'var(--color-text)' }}>
                  {selectedModel || 'Select model'}
                </span>
              </div>
              <ChevronDown size={12} style={{ color: 'var(--color-text-tertiary)' }} />
            </button>
          </div>

          {/* Conversation list */}
          <div className="flex-1 overflow-y-auto px-2">
            <ConversationList />
          </div>

          {/* Bottom nav */}
          <div className="px-2 pb-3 pt-2 flex flex-col" style={{ borderTop: '1px solid var(--color-border)' }}>
            <button
              onClick={() => updateSettings({ theme: settings.theme === 'dark' ? 'light' : 'dark' })}
              className="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors w-full text-left cursor-pointer"
              style={{ color: 'var(--color-text-secondary)' }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--color-bg-secondary)')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              {settings.theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
              {settings.theme === 'dark' ? 'Light mode' : 'Dark mode'}
            </button>
            {navItems.map((item) => {
              const isActive = location.pathname === item.path;
              return (
                <button
                  key={item.path}
                  onClick={() => navigate(item.path)}
                  className="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors w-full text-left cursor-pointer"
                  style={{
                    background: isActive ? 'var(--color-accent-subtle)' : 'transparent',
                    color: isActive ? 'var(--color-accent)' : 'var(--color-text-secondary)',
                    fontWeight: isActive ? 500 : 400,
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = isActive ? 'var(--color-accent-subtle)' : 'var(--color-bg-secondary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = isActive ? 'var(--color-accent-subtle)' : 'transparent';
                  }}
                >
                  <item.icon size={16} />
                  {item.label}
                </button>
              );
            })}
          </div>
        </div>
      </aside>
    </>
  );
}
