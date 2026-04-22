import { browser } from '$app/environment';
import { THEMES } from './themes';

function createColorTheme() {
	let themeId = $state<string>(
		browser ? (localStorage.getItem('colorTheme') ?? 'default') : 'default'
	);

	return {
		get id() {
			return themeId;
		},
		get theme() {
			return THEMES.find((t) => t.id === themeId) ?? THEMES[0];
		},
		set(id: string) {
			themeId = id;
			if (browser) localStorage.setItem('colorTheme', id);
		}
	};
}

export const colorTheme = createColorTheme();
