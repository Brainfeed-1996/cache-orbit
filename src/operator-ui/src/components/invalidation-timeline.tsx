'use client';

export function InvalidationTimeline() {
  const events = [
    { time: '14:32:01', key: 'user:1234:profile', action: 'invalidate', node: 'node-1' },
    { time: '14:32:00', key: 'session:*', action: 'invalidate_pattern', node: 'cluster' },
    { time: '14:31:58', key: 'product:5678:price', action: 'set', node: 'node-2' },
    { time: '14:31:55', key: 'user:1234:cart', action: 'invalidate', node: 'node-1' },
    { time: '14:31:52', key: 'hot:key:data', action: 'hotkey_replicated', node: 'node-3' },
  ];

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <h2 className="mb-4 text-lg font-semibold">Timeline des invalidations</h2>
      <div className="space-y-2">
        {events.map((e, i) => (
          <div key={i} className="flex items-center gap-4 border-l-2 border-slate-800 pl-4">
            <span className="font-mono text-xs text-slate-500">{e.time}</span>
            <span className={`rounded px-2 py-0.5 text-xs ${
              e.action === 'invalidate' ? 'bg-red-500/20 text-red-400' :
              e.action === 'set' ? 'bg-blue-500/20 text-blue-400' :
              'bg-yellow-500/20 text-yellow-400'
            }`}>
              {e.action}
            </span>
            <code className="text-sm text-slate-300">{e.key}</code>
            <span className="ml-auto text-xs text-slate-500">{e.node}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
