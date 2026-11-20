export const meta = {
  eyebrow: 'EXPERIMENTAL BACKEND',
  title: 'Ash — pure Vulkan, next to wgpu',
  date: '20 November 2026',
  version: '0.1.6',
  hardware: 'Intel(R) HD Graphics 620, Windows, debug build',
  resolution: '1200 x 800',
  scene: 'Raymarched 3D surface (sphere, torus, plane, soft shadows, AO, two reflections)'
};

export const highlights = [
  { value: '+34%', label: 'FPS on the heavy scene', detail: '21.8 vs 16.3 fps' },
  { value: '2.4x', label: 'Less CPU on the light scene', detail: '4.2% vs 10.1%' },
  { value: '−36%', label: 'Memory on the heavy scene', detail: '92.4 vs 144.8 MB' },
  { value: '~6%', label: 'Faster first frame', detail: '859 vs 916 ms' }
];

export const lightScene = [
  { metric: 'Frames per second', wgpu: 60.0, ash: 60.1, unit: 'fps', better: 'high' },
  { metric: 'CPU load', wgpu: 10.1, ash: 4.2, unit: '%', better: 'low' },
  { metric: 'Memory (working set)', wgpu: 136.6, ash: 94.8, unit: 'MB', better: 'low' },
  { metric: 'Frame cost (avg)', wgpu: 15.24, ash: 15.32, unit: 'ms', better: 'low' }
];

export const heavyScene = [
  { metric: 'Frames per second', wgpu: 16.3, ash: 21.8, unit: 'fps', better: 'high' },
  { metric: 'CPU load', wgpu: 8.6, ash: 0.2, unit: '%', better: 'low' },
  { metric: 'Memory (working set)', wgpu: 144.8, ash: 92.4, unit: 'MB', better: 'low' },
  { metric: 'Frame cost (avg)', wgpu: 60.28, ash: 44.9, unit: 'ms', better: 'low' }
];

export const general = [
  { metric: 'First frame', wgpu: 916, ash: 859, unit: 'ms', better: 'low' },
  { metric: 'CPU load, UI showcase', wgpu: 51.4, ash: 25.8, unit: '%', better: 'low' },
  { metric: 'Memory, UI showcase', wgpu: 148.2, ash: 109.4, unit: 'MB', better: 'low' },
  { metric: 'Frames per second, UI showcase', wgpu: 55.9, ash: 60.1, unit: 'fps', better: 'high' }
];

export const facts = [
  {
    title: 'One scene description, two backends',
    text: 'The built-in renderer WGSL module is shared. wgpu consumes it directly; the ash backend compiles the same source to SPIR-V with naga, so coverage maths and colour handling cannot drift apart.'
  },
  {
    title: 'An embeddable surface for Vulkan too',
    text: 'AshSurface mirrors WgpuSurface: implement AshSurfaceRenderer, receive the raw Vulkan device, queue, command buffer, render pass and target view, and record your own pipelines into the running UI.'
  },
  {
    title: 'Switch without changing code',
    text: 'App::render_backend(RenderBackend::VulkanAsh) selects the experimental backend. For quick A/B runs, SEQUOIA_RENDER_BACKEND=ash overrides the application setting; wgpu:vulkan keeps wgpu on the same API.'
  },
  {
    title: 'What is still missing',
    text: 'Backdrops, filters, isolated opacity groups and custom WGSL user shaders are not implemented in the Vulkan renderer yet. Use wgpu when those effects are required.'
  }
];

export const caveats = [
  'Measurements were taken on an integrated Intel HD 620 in a debug build; absolute numbers will differ on other GPUs.',
  'The light scene is vsync-capped at 60 fps on both backends, so only CPU and memory differ there.',
  'The heavy scene is GPU-bound well below vsync, which is where the ash backend shows its throughput advantage.',
  'The ash renderer currently uses one frame in flight; more in-flight frames are planned.'
];

/** @param {{ better: string, wgpu: number, ash: number }} row */
export function delta(row) {
  const ratio = row.better === 'low' ? row.wgpu / row.ash : row.ash / row.wgpu;
  const pct = Math.round((ratio - 1) * 100);
  return `${pct >= 0 ? '+' : ''}${pct}%`;
}

/** @param {{ better: string, wgpu: number, ash: number, unit: string }} row */
export function winner(row) {
  if (row.better === 'low') return row.ash < row.wgpu ? 'ash' : 'wgpu';
  return row.ash > row.wgpu ? 'ash' : 'wgpu';
}

/** @param {number} value */
export function num(value) {
  return value.toLocaleString('en-GB', { minimumFractionDigits: value % 1 ? 2 : 0 });
}
