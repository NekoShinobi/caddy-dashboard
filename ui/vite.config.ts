import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Only set when the dev server is reached through a TLS-terminating tunnel or
// reverse proxy, where the browser's port differs from Vite's. Hardcoding it
// breaks every plain-HTTP local run: the HMR socket dials the wrong port and
// never connects, and because the page still serves, the failure looks like
// "HMR stopped working" rather than an error. Unset, Vite infers it from the
// page origin, which is correct for local development.
//   HMR_CLIENT_PORT=443 just dev-ui
const hmrClientPort = process.env.HMR_CLIENT_PORT;

// On the host the API is a sibling process on localhost; under `just up-dev`
// it is a separate container, where localhost would be the Vite container
// itself. compose.dev.yml sets API_URL accordingly.
const apiTarget = process.env.API_URL ?? 'http://localhost:9080';

// Polling is required where inotify does not cross the mount (a dev container,
// Docker Desktop, a network filesystem) and is wasted CPU everywhere else, so
// it is opt-in rather than always on.
const usePolling = process.env.VITE_USE_POLLING === 'true';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	// `leaflet` and `leaflet.markercluster` are reached only through dynamic
	// imports in routes/map/+page.svelte, so Vite's dependency scanner never
	// sees them at startup. Without this the optimizer discovers them the first
	// time someone opens /map, which changes the dependency hash and turns every
	// module the open page already loaded into a 504.
	optimizeDeps: {
		include: ['leaflet', 'leaflet.markercluster']
	},
	server: {
		host: '0.0.0.0',
		allowedHosts: true,
		...(hmrClientPort ? { hmr: { clientPort: Number(hmrClientPort) } } : {}),
		watch: {
			usePolling,
			interval: 250,
			// A production build writes into these, and without the exclusion the
			// dev server fires a full reload for every file `bun run build` emits.
			ignored: ['**/build/**', '**/.svelte-kit/output/**', '**/.svelte-kit/generated/**']
		},
		headers: {
			'Cache-Control': 'no-store'
		},
		proxy: {
			'/api': {
				target: apiTarget,
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
