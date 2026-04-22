import { browser } from '$app/environment';

function createTheme() {
	let dark = $state(browser ? (localStorage.getItem('theme') ?? 'dark') === 'dark' : true);

	return {
		get dark() {
			return dark;
		},
		toggle() {
			dark = !dark;
			if (browser) localStorage.setItem('theme', dark ? 'dark' : 'light');
		}
	};
}

export const theme = createTheme();
