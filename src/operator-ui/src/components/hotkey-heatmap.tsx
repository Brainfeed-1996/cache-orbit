'use client';

interface HeatmapDatum {
  key: string;
  qps: number;
  p99_ms: number;
  hit_ratio: number;
  flagged: boolean;
}

const sample: HeatmapDatum[] = Array.from({ length: 36 }, (_, i) => ({
  key: `key-${i}`,
  qps: Math.floor(Math.random() * 60_000),
  p99_ms: Math.floor(Math.random() * 12),
  hit_ratio: +(Math.random() * 0.15 + 0.85).toFixed(3),
  flagged: Math.random() > 0.83,
}));

export function HotKeyHeatmap() {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <h2 className="mb-4 text-lg font-semibold">Heatmap du keyspace</h2>
      <div className="grid grid-cols-6 gap-1 md:grid-cols-9">
        {sample.map((d) => (
          <div
            key={d.key}
            title={`${d.key}: ${d.qps.toLocaleString('fr-FR')} req/s`}
            className={`relative h-8 w-full rounded ${d.flagged ? 'border-2 border-red-500' : ''}`}
            style={{
              backgroundColor: `rgba(59,130,246,${d.qps / 60_000 * 0.9})`,
            }}
          />
        ))}
      </div>
      <div className="mt-4">
        <table className="w-full table-auto text-left text-sm">
          <thead>
            <tr className="text-slate-400">
              <th className="px-2 py-1">Clé</th>
              <th className="px-2 py-1">QPS</th>
              <th className="px-2 py-1">P99</th>
              <th className="px-2 py-1">Remplis.</th>
            </tr>
          </thead>
          <tbody>
            {sample.filter((d) => d.flagged).slice(0, 5).map((d) => (
              <tr key={d.key} className="border-t border-slate-800">
                <td className="px-2 py-1 font-mono text-red-400">{d.key}</td>
                <td className="px-2 py-1">{d.qps.toLocaleString('fr-FR')}</td>
                <td className="px-2 py-1">{d.p99_ms}ms</td>
                <td className="px-2 py-1">{(d.hit_ratio * 100).toFixed(1)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
