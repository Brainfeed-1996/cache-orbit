'use client';

export function BenchmarkPanel() {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 p-4">
      <h2 className="mb-4 text-lg font-semibold">Benchmark</h2>
      <div className="space-y-3">
        <div>
          <label className="mb-1 block text-xs text-slate-400">Scénario</label>
          <select className="w-full rounded border border-slate-800 bg-slate-900 p-2 text-sm">
            <option>read-heavy</option>
            <option>write-heavy</option>
            <option>burst</option>
            <option>scan-pattern</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-slate-400">Requêtes</label>
          <input type="number" defaultValue={100000} className="w-full rounded border border-slate-800 bg-slate-900 p-2 text-sm" />
        </div>
        <div>
          <label className="mb-1 block text-xs text-slate-400">Concurrency</label>
          <input type="number" defaultValue={128} className="w-full rounded border border-slate-800 bg-slate-900 p-2 text-sm" />
        </div>
        <button className="w-full rounded bg-blue-600 py-2 text-sm font-semibold hover:bg-blue-500">
          Lancer le benchmark
        </button>
      </div>
      <div className="mt-4 space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">Dernier run</span>
          <span className="text-emerald-400">✔ read-heavy</span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">Throughput</span>
          <span className="font-mono">1.2M ops/s</span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">P99</span>
          <span className="font-mono">4.2ms</span>
        </div>
      </div>
    </div>
  );
}
