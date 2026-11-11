/**
 * @param {HTMLElement} node
 * @param {{ y?: number, duration?: number, delay?: number, threshold?: number, once?: boolean }} [options]
 * @returns {{ destroy?: () => void }}
 */
export function reveal(node, options = {}) {
	const { y = 32, duration = 700, delay = 0, threshold = 0.15, once = true } = options;

	const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	if (reduced) {
		node.style.opacity = '1';
		return {};
	}

	node.style.opacity = '0';
	node.style.transform = `translateY(${y}px)`;
	node.style.transition =
		`opacity ${duration}ms ease ${delay}ms, ` +
		`transform ${duration}ms cubic-bezier(.16,1,.3,1) ${delay}ms`;
	node.style.willChange = 'opacity, transform';

	const io = new IntersectionObserver(
		([entry]) => {
			if (entry.isIntersecting) {
				node.style.opacity = '1';
				node.style.transform = 'translateY(0)';
				if (once) io.unobserve(node);
			} else if (!once) {
				node.style.opacity = '0';
				node.style.transform = `translateY(${y}px)`;
			}
		},
		{ threshold }
	);

	io.observe(node);
	return { destroy: () => io.disconnect() };
}