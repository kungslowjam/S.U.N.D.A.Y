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
  Mic,
  Brain,
  Zap,
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
    { path: '/command-center', icon: Zap, label: 'Mission Control' },
    { path: '/dashboard', icon: BarChart3, label: 'Dashboard' },
    { path: 'http://127.0.0.1:8098', icon: Mic, label: 'Voice Live', external: true },
    { path: '/data-sources', icon: Brain, label: 'Knowledge & Brain' },
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
          className="fixed top-3 left-3 z-30 p-2 rounded-lg transition-all cursor-pointer"
          style={{ color: '#9ca3af', background: 'rgba(30, 30, 30, 0.8)' }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'rgba(59, 130, 246, 0.2)';
            e.currentTarget.style.color = '#60a5f9';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'rgba(30, 30, 30, 0.8)';
            e.currentTarget.style.color = '#9ca3af';
          }}
        >
          <PanelLeft size={18} />
        </button>
      )}

      <aside
        className={`
          flex flex-col h-full shrink-0 transition-all duration-300 ease-in-out overflow-hidden
          fixed md:relative z-30
          ${sidebarOpen ? 'w-[260px]' : 'w-0'}
        `}
        style={{
          background: 'rgba(17, 17, 17, 0.95)',
          backdropFilter: 'blur(20px)',
          borderRight: sidebarOpen ? '1px solid rgba(255, 255, 255, 0.06)' : 'none',
        }}
      >
        <div className="flex flex-col h-full w-[260px]">
          <div className="flex items-center justify-between px-3 pt-3 pb-2">
            <button
              onClick={toggleSidebar}
              className="p-2 rounded-lg transition-all cursor-pointer"
              style={{ color: '#9ca3af' }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(59, 130, 246, 0.15)';
                e.currentTarget.style.color = '#60a5f9';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.color = '#9ca3af';
              }}
            >
              <PanelLeftClose size={18} />
            </button>
            <button
              onClick={handleNewChat}
              className="p-2 rounded-lg transition-all cursor-pointer"
              style={{ color: '#9ca3af' }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(59, 130, 246, 0.15)';
                e.currentTarget.style.color = '#60a5f9';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.color = '#9ca3af';
              }}
              title="New chat"
            >
              <Plus size={18} />
            </button>
          </div>

          <div className="px-3 mb-3">
            <button
              onClick={handleNewChat}
              className="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm transition-all cursor-pointer"
              style={{
                background: 'linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(37, 99, 235, 0.15) 100%)',
                color: '#e5e7eb',
                border: '1px solid rgba(59, 130, 246, 0.3)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'linear-gradient(135deg, rgba(59, 130, 246, 0.3) 0%, rgba(37, 99, 235, 0.25) 100%)';
                e.currentTarget.style.boxShadow = '0 4px 15px rgba(59, 130, 246, 0.3)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(37, 99, 235, 0.15) 100%)';
                e.currentTarget.style.boxShadow = 'none';
              }}
            >
              <Plus size={16} />
              New chat
            </button>
          </div>

          <div className="px-3 mb-3">
            <button
              onClick={() => setCommandPaletteOpen(true)}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-xl text-xs transition-all cursor-pointer"
              style={{
                background: 'rgba(30, 30, 30, 0.6)',
                color: '#9ca3af',
                border: '1px solid rgba(255, 255, 255, 0.06)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(45, 45, 45, 0.8)';
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.1)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'rgba(30, 30, 30, 0.6)';
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.06)';
              }}
            >
              {modelLoading ? (
                <Loader2 size={14} className="animate-spin" style={{ color: '#60a5f9' }} />
              ) : (
                <Cpu size={14} />
              )}
              <div className="flex-1 min-w-0 text-left">
                <span className="truncate block" style={{ color: '#e5e7eb' }}>
                  {selectedModel || 'Select model'}
                </span>
              </div>
              <ChevronDown size={12} style={{ color: '#6b7280' }} />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto px-2">
            <ConversationList />
          </div>

          <div className="px-2 pb-3 pt-2 flex flex-col" style={{ borderTop: '1px solid rgba(255, 255, 255, 0.06)' }}>
            <button
              onClick={() => updateSettings({ theme: settings.theme === 'dark' ? 'light' : 'dark' })}
              className="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all w-full text-left cursor-pointer"
              style={{ color: '#9ca3af' }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
              }}
            >
              {settings.theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
              {settings.theme === 'dark' ? 'Light mode' : 'Dark mode'}
            </button>
            {navItems.map((item) => {
              const isActive = !item.external && location.pathname === item.path;
              return (
                <button
                  key={item.path}
                  onClick={() => {
                    if (item.external) {
                      window.open(item.path, 'sunday-voice-live', 'noopener,noreferrer');
                    } else {
                      navigate(item.path);
                    }
                  }}
                  className="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all w-full text-left cursor-pointer"
                  style={{
                    background: isActive ? 'rgba(59, 130, 246, 0.15)' : 'transparent',
                    color: isActive ? '#60a5f9' : '#9ca3af',
                    fontWeight: isActive ? 500 : 400,
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = isActive ? 'rgba(59, 130, 246, 0.15)' : 'rgba(59, 130, 246, 0.1)';
                    e.currentTarget.style.color = '#60a5f9';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = isActive ? 'rgba(59, 130, 246, 0.15)' : 'transparent';
                    e.currentTarget.style.color = isActive ? '#60a5f9' : '#9ca3af';
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
