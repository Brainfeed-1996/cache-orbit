'use client';
import { MetricsDashboard } from '@/components/metrics-dashboard';
import { HotKeyHeatmap } from '@/components/hotkey-heatmap';
import { InvalidationTimeline } from '@/components/invalidation-timeline';
import { BenchmarkPanel } from '@/components/benchmark-panel';

export default function OperatorConsole() {
  return (
    <main className="min-h-screen bg-slate-950 text-slate-100 font-mono">
      <DashboardHeader />
      <div className="grid grid-cols-12 gap-4 p-6">
        <section className="col-span-12 lg:col-span-6">
          <MetricsDashboard />
        </section>
        <section className="col-span-12 lg:col-span-6">
          <HotKeyHeatmap />
        </section>
        <section className="col-span-12 lg:col-span-8">
          <InvalidationTimeline />
        </section>
        <section className="col-span-12 lg:col-span-4">
          <BenchmarkPanel />
        </section>
      </div>
    </main>
  );
}

function DashboardHeader() {
  return (
    <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur">
      <div className="mx-auto flex max-w-7xl items-center justify-between px-6 py-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Cache Orbit</h1>
          <p className="text-sm text-slate-400">
            control-plane: <span className="text-emerald-400">healthy</span> · nodes: 4/4 · uptime: 14d 7h 23m
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="inline-flex h-2 w-2 animate-pulse rounded-full bg-emerald-500" />
          <span className="text-sm text-slate-300">Live</span>
        </div>
      </div>
    </header>
  );
}
