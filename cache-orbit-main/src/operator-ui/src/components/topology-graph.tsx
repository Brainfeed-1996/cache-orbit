'use client';
import { useState } from 'react';

export default function CacheTopologyGraph() {
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const nodes = [
    { id: 'node-1', datacenter: 'eu-west-1', state: 'healthy', qps: 45_000 },
    { id: 'node-2', datacenter: 'eu-west-1', state: 'healthy', qps: 39_000 },
    { id: 'node-3', datacenter: 'us-east-1', state: 'healthy', qps: 51_000 },
    { id: 'node-4', datacenter: 'us-east-1', state: 'healthy', qps: 47_000 },
  ];
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Topologie de cluster</h2>
        <div className="flex gap-2">
          <div className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full bg-emerald-500" />
            <span className="text-xs text-slate-400">sain</span>
          </div>
          <div className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full bg-yellow-500" />
            <span className="text-xs text-slate-400">dégradé</span>
          </div>
          <div className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full bg-red-500" />
            <span className="text-xs text-slate-400">critique</span>
          </div>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        {nodes.map((n) => (
          <div
            key={n.id}
            className={`rounded border p-3 ${
              n.state === 'healthy'
                ? 'border-slate-800 bg-slate-900'
                : n.state === 'degraded'
                ? 'border-yellow-500/40 bg-yellow-500/5'
                : 'border-red-500/40 bg-red-500/5'
            }`}
            onMouseEnter={() => setHoveredNode(n.id)}
            onMouseLeave={() => setHoveredNode(null)}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className={`inline-block h-2 w-2 rounded-full ${n.state === 'healthy' ? 'bg-emerald-500' : 'bg-yellow-500'}`} />
                <span className="font-mono text-sm">{n.id}</span>
              </div>
              <span className="text-xs text-slate-400">{n.datacenter}</span>
            </div>
            <div className="mt-2 flex justify-between text-xs text-slate-400">
              <span className="font-mono">{n.qps.toLocaleString('fr-FR')} req/s</span>
              <span className={n.state === 'healthy' ? 'text-emerald-400' : 'text-yellow-400'}>
                {Math.floor((Math.random() * 5 + 95))}.{Math.floor(Math.random() * 9)}% uptime
              </span>
            </div>
          </div>
        ))}
      </div>
      <div className="mt-4 flex items-center justify-between">
        <p className="text-xs text-slate-500">1,024 partons · réplicas actifs: 8 · mémoire: 71.4GB / 128GB</p>
        <span className="text-xs text-slate-500">v{Math.floor(Math.random() * 20 + 120)}.{Math.floor(Math.random() * 10)} {new Date().toISOString().split('T')[0]}</span>
      </div>
    </div>
  );
}
