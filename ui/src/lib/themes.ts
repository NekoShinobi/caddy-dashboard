export interface ChartColors {
	green: string;
	yellow: string;
	red: string;
	blue: string;
	purple: string;
	orange: string;
}

export interface ColorTheme {
	id: string;
	name: string;
	preview: string;
	dark: ChartColors;
	light: ChartColors;
}

export const THEMES: ColorTheme[] = [
	{
		id: 'default',
		name: 'Default',
		preview: '#60a5fa',
		dark:  { green: '#4ade80', yellow: '#facc15', red: '#f87171', blue: '#60a5fa', purple: '#c084fc', orange: '#fb923c' },
		light: { green: '#16a34a', yellow: '#ca8a04', red: '#dc2626', blue: '#2563eb', purple: '#9333ea', orange: '#ea580c' }
	},
	{
		id: 'nord',
		name: 'Nord',
		preview: '#88c0d0',
		dark:  { green: '#a3be8c', yellow: '#ebcb8b', red: '#bf616a', blue: '#88c0d0', purple: '#b48ead', orange: '#d08770' },
		light: { green: '#4a7c59', yellow: '#9a7700', red: '#922020', blue: '#2e7a94', purple: '#7a5a8a', orange: '#9a4a20' }
	},
	{
		id: 'dracula',
		name: 'Dracula',
		preview: '#bd93f9',
		dark:  { green: '#50fa7b', yellow: '#f1fa8c', red: '#ff5555', blue: '#8be9fd', purple: '#bd93f9', orange: '#ffb86c' },
		light: { green: '#0d7c35', yellow: '#9a9a00', red: '#cc0000', blue: '#0077aa', purple: '#6c3db9', orange: '#cc6600' }
	},
	{
		id: 'catppuccin',
		name: 'Catppuccin',
		preview: '#cba6f7',
		dark:  { green: '#a6e3a1', yellow: '#f9e2af', red: '#f38ba8', blue: '#89b4fa', purple: '#cba6f7', orange: '#fab387' },
		light: { green: '#40a02b', yellow: '#df8e1d', red: '#d20f39', blue: '#1e66f5', purple: '#8839ef', orange: '#fe640b' }
	},
	{
		id: 'sunset',
		name: 'Sunset',
		preview: '#f97316',
		dark:  { green: '#fb923c', yellow: '#fbbf24', red: '#f43f5e', blue: '#a78bfa', purple: '#e879f9', orange: '#f59e0b' },
		light: { green: '#c2410c', yellow: '#b45309', red: '#be123c', blue: '#7c3aed', purple: '#a21caf', orange: '#92400e' }
	},
	{
		id: 'neon',
		name: 'Neon',
		preview: '#00f5ff',
		dark:  { green: '#39ff14', yellow: '#ffff00', red: '#ff2079', blue: '#00f5ff', purple: '#bf00ff', orange: '#ff6600' },
		light: { green: '#1a8500', yellow: '#9a9400', red: '#cc0044', blue: '#0077aa', purple: '#6600aa', orange: '#cc4400' }
	}
];
