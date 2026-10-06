import { mount } from 'svelte';
import './theme.css';
import App from './App.svelte';
import { loadSettings, prefs } from './lib/ui.svelte.js';
import { applyAccent } from './lib/accent.js';

mount(App, { target: document.getElementById('app') });

// Settings live in /data/adb/kmgr/settings.json; load them and re-theme.
loadSettings().then(() => applyAccent(prefs.accent));

