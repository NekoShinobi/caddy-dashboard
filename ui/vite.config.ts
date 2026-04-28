import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		host: '0.0.0.0',
		allowedHosts: true,
		hmr: {
			clientPort: 443
		},
		headers: {
			'Cache-Control': 'no-store'
		},
		proxy: {
			'/api': {
				target: 'http://localhost:9080',
				changeOrigin: true,
				configure: (proxy) => {
					proxy.on('proxyRes', (_proxyRes, _req, res) => {
						res.setHeader('Cache-Control', 'no-store');
					});
				}
			}
		}
	}
});
