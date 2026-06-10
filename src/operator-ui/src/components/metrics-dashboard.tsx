'use client';

interface MetricCard {
  label: string;
  value: number;
  unit: string;
  change?: number;
  limit?: number;
}

export function MetricsDashboard() {
  const metrics: MetricCard[] = [
    {
      label: 'Hit ratio',
      value: 94.2,
      unit: '%',
      change: 0.3,
      limit: 99,
    },
    {
      label: 'Latence P50',
      value: 0.8,
      unit: 'ms',
      change: 0.1,
      limit: 5,
    },
    {
      label: 'Latence P99',
      value: 4.2,
      unit: 'ms',
      change: -0.4,
      limit: 10,
    },
    {
      label: 'Invalidations',
      value: 1820,
      unit: '/s',
      change: 0,
      limit: 5000,
    },
    {
      label: 'Pression backend',
      value: 320,
      unit: 'req/s',
      change: 12,
      limit: 1000,
    },
    {
      label: 'Staleness',
      value: 12,
      unit: 'ms',
      change: -2,
      limit: 500,
    },
  ];

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <h2 className="mb-4 text-lg font-semibold">Métriques clés</h2>
      <div className="grid grid-cols-2 gap-4">
        {metrics.map((m) => {
          const pct = typeof m.value === 'number' && m.limit ? Math.min((m.value / m.limit) * 100, 100) : 0;
          const colorClass = pct > 80 ? 'text-amber-400' : pct > 90 ? 'text-emerald-400' : 'text-slate-300';
          return (
            <div key={m.label} className="rounded border border-slate-800 bg-slate-900 p-3">
              <p className="text-xs text-slate-400">{m.label}</p>
              <p className={`text-2xl font-bold ${colorClass}`}>
                {m.value.toLocaleString('fr-FR')}
                <span className="ml-1 text-sm font-normal text-slate-400">{m.unit}</span>
              </p>
              {m.change !== undefined && (
                <p className={`text-xs ${m.change > 0 ? 'text-emerald-400' : m.change < 0 ? 'text-red-400' : 'text-slate-400'}`}>
                  {m.change > 0 ? '+' : ''}{m.change}% vs baseline
                </p>
              )}
              <div className="mt-2 h-1 overflow-hidden rounded bg-slate-800">
                <div className={`h-full ${pct > 85 ? 'bg-amber-500' : 'bg-blue-500'}`} style={{ width: `${pct}%` }} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
