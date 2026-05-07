interface Props {
  phase: string;
}

export function StreamingDots({ phase }: Props) {
  return (
    <div className="flex items-center gap-3 py-3">
      <div className="flex gap-1.5">
        <span
          className="w-2 h-2 rounded-full animate-pulse"
          style={{ 
            background: '#60a5f9', 
            animationDelay: '0ms',
            boxShadow: '0 0 8px rgba(96, 165, 249, 0.5)',
          }}
        />
        <span
          className="w-2 h-2 rounded-full animate-pulse"
          style={{ 
            background: '#60a5f9', 
            animationDelay: '200ms',
            boxShadow: '0 0 8px rgba(96, 165, 249, 0.5)',
          }}
        />
        <span
          className="w-2 h-2 rounded-full animate-pulse"
          style={{ 
            background: '#60a5f9', 
            animationDelay: '400ms',
            boxShadow: '0 0 8px rgba(96, 165, 249, 0.5)',
          }}
        />
      </div>
      {phase && (
        <span className="text-xs" style={{ color: '#9ca3af' }}>
          {phase}
        </span>
      )}
    </div>
  );
}
