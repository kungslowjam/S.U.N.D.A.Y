import { useState } from 'react';
import { useNavigate } from 'react-router';
import { Trash2, MessageSquare } from 'lucide-react';
import { useAppStore } from '../../lib/store';

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(timestamp).toLocaleDateString();
}

export function ConversationList() {
  const navigate = useNavigate();
  const conversations = useAppStore((s) => s.conversations);
  const activeId = useAppStore((s) => s.activeId);
  const selectConversation = useAppStore((s) => s.selectConversation);
  const deleteConversation = useAppStore((s) => s.deleteConversation);
  const [searchQuery, setSearchQuery] = useState('');

  const filtered = searchQuery
    ? conversations.filter((c) =>
        c.title.toLowerCase().includes(searchQuery.toLowerCase()),
      )
    : conversations;

  return (
    <div className="flex flex-col h-full">
      <div className="px-1 mb-2">
        <div
          className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-sm"
          style={{ background: 'var(--color-bg-secondary)', border: '1px solid var(--color-border)' }}
        >
          <MessageSquare size={12} style={{ color: 'var(--color-text-tertiary)' }} />
          <input
            type="text"
            placeholder="Search chats..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 bg-transparent outline-none text-xs"
            style={{ color: 'var(--color-text)' }}
          />
        </div>
      </div>

      <div className="text-[11px] px-3 py-1 font-medium" style={{ color: 'var(--color-text-tertiary)' }}>
        Today
      </div>

      {filtered.length === 0 ? (
        <div className="px-3 py-6 text-center text-xs" style={{ color: 'var(--color-text-tertiary)' }}>
          {searchQuery ? 'No matching chats' : 'No conversations yet'}
        </div>
      ) : (
        <div className="flex flex-col gap-0.5 pb-2">
          {filtered.map((conv) => {
            const isActive = conv.id === activeId;
            return (
              <div
                key={conv.id}
                className="group flex items-center rounded-lg cursor-pointer transition-colors"
                style={{ background: isActive ? 'var(--color-bg-secondary)' : 'transparent' }}
                onMouseEnter={(e) => {
                  if (!isActive) e.currentTarget.style.background = 'var(--color-bg-secondary)';
                }}
                onMouseLeave={(e) => {
                  if (!isActive) e.currentTarget.style.background = 'transparent';
                }}
              >
                <button
                  onClick={() => {
                    selectConversation(conv.id);
                    navigate('/');
                  }}
                  className="flex-1 text-left px-3 py-2.5 min-w-0 cursor-pointer"
                >
                  <div
                    className="text-sm truncate"
                    style={{
                      color: isActive ? 'var(--color-text)' : 'var(--color-text-secondary)',
                      fontWeight: isActive ? 500 : 400,
                    }}
                  >
                    {conv.title}
                  </div>
                  <div className="text-[11px] mt-0.5" style={{ color: 'var(--color-text-tertiary)' }}>
                    {formatRelativeTime(conv.updatedAt)}
                  </div>
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteConversation(conv.id);
                  }}
                  className="p-1.5 mr-1 rounded opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
                  style={{ color: 'var(--color-text-tertiary)' }}
                  onMouseEnter={(e) => (e.currentTarget.style.color = 'var(--color-error)')}
                  onMouseLeave={(e) => (e.currentTarget.style.color = 'var(--color-text-tertiary)')}
                  title="Delete conversation"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
