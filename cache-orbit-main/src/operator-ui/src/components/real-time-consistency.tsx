'use client';

export function RealTimeConsistency() {
  const metrics = [
    { label: 'Staleness avg', value: 12, unit: 'ms' },
    { label: 'Clés cohérentes', value: 99.98, unit: '%' },
    { label: 'Invalidations en vol', value: 47, unit: '' },
    { label: 'Backlog NATS', value: 1203, unit: 'events' },
  ];

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <h2 className="mb-4 text-lg font-semibold">Cohérence en temps réel</h2>
      <div className="grid grid-cols-2 gap-4">
        {metrics.map((m) => (
          <div key={m.label} className="rounded border border-slate-800 bg-slate-900 p-3">
            <p className="text-xs text-slate-400">{m.label}</p>
            <p className="text-2xl font-bold">
              {typeof m.value === 'number' && m.value < 100 && m.unit === '%' ? m.value.toFixed(2) : m.value.toLocaleString('fr-FR')}
              <span className="ml-1 text-sm font-normal text-slate-400">{m.unit}</span>
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
