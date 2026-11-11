/**
 * @param {HTMLElement} node
 * @param {{ duration?: number, delay?: number }} [options]
 * @returns {{ destroy?: () => void }}
 */
export function softReveal(node, { duration = 900, delay = 0 } = {}) {
	if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return {};

	node.style.opacity = '0';
	node.style.filter = 'blur(10px)';
	node.style.transform = 'scale(0.985)';
	node.style.transition =
		`opacity ${duration}ms ease ${delay}ms, ` +
		`filter ${duration}ms ease ${delay}ms, ` +
		`transform ${duration}ms cubic-bezier(.2,.8,.2,1) ${delay}ms`;
	node.style.willChange = 'opacity, filter, transform';

	const io = new IntersectionObserver(
		([e]) => {
			if (!e.isIntersecting) return;
			node.style.opacity = '1';
			node.style.filter = 'blur(0)';
			node.style.transform = 'scale(1)';
			io.disconnect();
		},
		{ threshold: 0.2 }
	);
	io.observe(node);

	return { destroy: () => io.disconnect() };
}