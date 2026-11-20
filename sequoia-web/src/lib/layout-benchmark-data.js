export const layoutMeta = {
	sequoia: '0.1.6',
	taffy: '0.15.1',
	machine: 'Windows 11 · 4 logical CPUs',
	profile: 'release-bench (opt-level 3, thin LTO)',
	harness: 'cargo test -p sequoia-test --test layout_vs_taffy -- --nocapture',
	warmup: 20,
	warmFrames: 200,
	coldFrames: 60,
	when: '2026-11-20'
};

export const layoutRows = [
	{ kind: 'flex', mode: 'warm', nodes: 100, sequoia: [0.184, 0.175, 0.251], taffy: [0.157, 0.137, 0.233] },
	{ kind: 'flex', mode: 'warm', nodes: 1000, sequoia: [0.265, 0.273, 0.345], taffy: [1.623, 1.573, 2.71] },
	{ kind: 'flex', mode: 'warm', nodes: 3000, sequoia: [0.439, 0.424, 0.651], taffy: [7.269, 6.667, 21.262] },
	{ kind: 'flex', mode: 'cold', nodes: 100, sequoia: [1.325, 1.219, 1.614], taffy: [2.073, 2.049, 2.337] },
	{ kind: 'flex', mode: 'cold', nodes: 1000, sequoia: [28.652, 28.21, 34.74], taffy: [10.422, 10.539, 13.378] },
	{ kind: 'flex', mode: 'cold', nodes: 3000, sequoia: [92.418, 87.652, 145.218], taffy: [22.489, 22.132, 25.135] },
	{ kind: 'grid', mode: 'warm', nodes: 100, sequoia: [0.115, 0.111, 0.163], taffy: [0.058, 0.052, 0.077] },
	{ kind: 'grid', mode: 'warm', nodes: 1000, sequoia: [0.618, 0.403, 1.401], taffy: [0.561, 0.512, 0.908] },
	{ kind: 'grid', mode: 'warm', nodes: 3000, sequoia: [1.455, 1.306, 2.391], taffy: [2.009, 1.817, 3.081] },
	{ kind: 'grid', mode: 'cold', nodes: 100, sequoia: [0.434, 0.463, 0.491], taffy: [0.192, 0.192, 0.227] },
	{ kind: 'grid', mode: 'cold', nodes: 1000, sequoia: [9.958, 9.783, 12.481], taffy: [1.263, 1.146, 2.051] },
	{ kind: 'grid', mode: 'cold', nodes: 3000, sequoia: [30.802, 30.307, 36.859], taffy: [4.653, 4.429, 5.973] },
	{ kind: 'wrap', mode: 'warm', nodes: 100, sequoia: [0.085, 0.062, 0.155], taffy: [0.045, 0.029, 0.087] },
	{ kind: 'wrap', mode: 'warm', nodes: 1000, sequoia: [0.699, 0.43, 1.543], taffy: [0.295, 0.268, 0.498] },
	{ kind: 'wrap', mode: 'warm', nodes: 3000, sequoia: [4.14, 3.344, 10.61], taffy: [1.901, 1.868, 3.245] },
	{ kind: 'wrap', mode: 'cold', nodes: 100, sequoia: [0.478, 0.483, 0.61], taffy: [0.147, 0.143, 0.174] },
	{ kind: 'wrap', mode: 'cold', nodes: 1000, sequoia: [2.413, 1.915, 4.695], taffy: [1.044, 0.949, 1.542] },
	{ kind: 'wrap', mode: 'cold', nodes: 3000, sequoia: [9.043, 8.508, 14.315], taffy: [6.929, 6.659, 10.391] },
	{ kind: 'text', mode: 'warm', nodes: 100, sequoia: [0.038, 0.034, 0.062], taffy: [0.07, 0.055, 0.081] },
	{ kind: 'text', mode: 'warm', nodes: 1000, sequoia: [0.107, 0.083, 0.156], taffy: [0.507, 0.484, 0.898] },
	{ kind: 'text', mode: 'warm', nodes: 3000, sequoia: [0.163, 0.103, 0.349], taffy: [1.512, 1.227, 2.789] },
	{ kind: 'text', mode: 'cold', nodes: 100, sequoia: [0.496, 0.387, 2.114], taffy: [0.141, 0.143, 0.187] },
	{ kind: 'text', mode: 'cold', nodes: 1000, sequoia: [1.491, 1.639, 2.494], taffy: [1.205, 1.104, 1.798] },
	{ kind: 'text', mode: 'cold', nodes: 3000, sequoia: [2.74, 2.431, 4.928], taffy: [4.608, 4.258, 5.888] }
];
