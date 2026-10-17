<script lang="ts" module>
  // Stroke icons on a 24px grid, drawn in the style of Lucide (ISC).
  const paths: Record<string, string> = {
    "arrow-up-right": '<path d="M7 17 17 7"/><path d="M8 7h9v9"/>',
    refresh: '<path d="M21 12a9 9 0 0 1-15.5 6.2L3 16"/><path d="M3 21v-5h5"/><path d="M3 12a9 9 0 0 1 15.5-6.2L21 8"/><path d="M21 3v5h-5"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    copy: '<rect x="9" y="9" width="12" height="12" rx="2.5"/><path d="M5 15H4.5A1.5 1.5 0 0 1 3 13.5v-9A1.5 1.5 0 0 1 4.5 3h9A1.5 1.5 0 0 1 15 4.5V5"/>',
    key: '<circle cx="7.5" cy="15.5" r="4.5"/><path d="m10.7 12.3 9.8-9.8"/><path d="m17 6 3 3"/><path d="m14.5 8.5 2 2"/>',
    logout: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5"/><path d="M21 12H9"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
    commit: '<circle cx="12" cy="12" r="3.5"/><path d="M3 12h5.5"/><path d="M15.5 12H21"/>',
    branch: '<circle cx="6" cy="5" r="2.2"/><circle cx="6" cy="19" r="2.2"/><circle cx="18" cy="7" r="2.2"/><path d="M6 7.2v9.6"/><path d="M18 9.2c0 4.8-12 3.3-12 7.6"/>',
    github: '<path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"/><path d="M9 18c-4.51 2-5-2-7-2"/>',
    bookmark: '<path d="M19 21 12 16.5 5 21V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"/>',
    house: '<path d="M3 10.5 12 3l9 7.5"/><path d="M5 9v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V9"/><path d="M10 21v-6h4v6"/>',
    todo: '<rect x="3" y="3" width="18" height="18" rx="4"/><path d="m8 12 3 3 5-6"/>',
    network: '<rect x="9" y="2.5" width="6" height="5" rx="1.2"/><rect x="2.5" y="16.5" width="6" height="5" rx="1.2"/><rect x="15.5" y="16.5" width="6" height="5" rx="1.2"/><path d="M12 7.5V12"/><path d="M5.5 16.5V14a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v2.5"/>',
    server: '<rect x="3" y="3" width="18" height="7.5" rx="2"/><rect x="3" y="13.5" width="18" height="7.5" rx="2"/><path d="M7 6.75h.01"/><path d="M7 17.25h.01"/>',
    laptop: '<rect x="4" y="4" width="16" height="11" rx="1.8"/><path d="M2 19.5h20"/>',
    desktop: '<rect x="2.5" y="3.5" width="19" height="13" rx="2"/><path d="M8.5 20.5h7"/><path d="M12 16.5v4"/>',
    router: '<rect x="2.5" y="13.5" width="19" height="7" rx="2"/><path d="M6.5 17h.01"/><path d="M10.5 17h.01"/><path d="M15 13.5 18 7"/><path d="M15.6 5.2a4.5 4.5 0 0 1 4.8 2.2"/>',
    phone: '<rect x="6" y="2.5" width="12" height="19" rx="2.5"/><path d="M11 18h2"/>',
    chip: '<rect x="5" y="5" width="14" height="14" rx="2.5"/><rect x="9" y="9" width="6" height="6" rx="1"/><path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3"/>',
    nas: '<rect x="4" y="2.5" width="16" height="19" rx="2.5"/><path d="M8 7h8"/><path d="M8 11h8"/><path d="M8 17.5h.01"/><path d="M12 17.5h.01"/>',
    thermometer: '<path d="M14 14.8V4.5a2 2 0 1 0-4 0v10.3a4 4 0 1 0 4 0z"/>',
    droplet: '<path d="M12 21.5a6.5 6.5 0 0 0 6.5-6.5c0-3.5-3-6.5-6.5-12C8.5 8.5 5.5 11.5 5.5 15a6.5 6.5 0 0 0 6.5 6.5z"/>',
    bolt: '<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>',
    bulb: '<path d="M9 18h6"/><path d="M10 21.5h4"/><path d="M12 2.5a6.5 6.5 0 0 0-4 11.6c.7.6 1 1.4 1 2.4v.5h6v-.5c0-1 .3-1.8 1-2.4a6.5 6.5 0 0 0-4-11.6z"/>',
    power: '<path d="M12 2.5v9"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/>',
    wind: '<path d="M3 8h11a3 3 0 1 0-3-3"/><path d="M3 12h16a3 3 0 1 1-3 3"/><path d="M3 16h7"/>',
    door: '<path d="M5 21V4.5A1.5 1.5 0 0 1 6.5 3h11A1.5 1.5 0 0 1 19 4.5V21"/><path d="M3 21h18"/><path d="M14.5 12h.01"/>',
    battery: '<rect x="2.5" y="7" width="16" height="10" rx="2.2"/><path d="M21.5 11v2"/><path d="M6 10.5v3"/><path d="M9.5 10.5v3"/>',
    star: '<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"/>',
    fork: '<circle cx="6" cy="5" r="2.2"/><circle cx="18" cy="5" r="2.2"/><circle cx="12" cy="19" r="2.2"/><path d="M6 7.2V9a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7.2"/><path d="M12 11v5.8"/>',
    alert: '<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5"/><path d="M12 16.2h.01"/>',
    notebook: '<path d="M6 3h11.5A1.5 1.5 0 0 1 19 4.5v15a1.5 1.5 0 0 1-1.5 1.5H6"/><path d="M6 3v18"/><path d="M3.5 7.5h5"/><path d="M3.5 12h5"/><path d="M3.5 16.5h5"/><path d="M11.5 8h4.5"/>',
    box: '<path d="M21 8 12 3 3 8v8l9 5 9-5z"/><path d="m3 8 9 5 9-5"/><path d="M12 13v8"/>',
    globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a14 14 0 0 1 0 18"/><path d="M12 3a14 14 0 0 0 0 18"/>',
    chevron: '<path d="m9 6 6 6-6 6"/>',
    circle: '<circle cx="12" cy="12" r="8.5"/>',
    calendar: '<rect x="3" y="4.5" width="18" height="16.5" rx="2.5"/><path d="M3 9.5h18"/><path d="M8 2.5v4"/><path d="M16 2.5v4"/>',
    inbox: '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>',
    image: '<rect x="3" y="3" width="18" height="18" rx="2.5"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.6-3.6a2 2 0 0 0-2.8 0L5 21"/>',
    file: '<path d="M14 2.5H6.5A1.5 1.5 0 0 0 5 4v16a1.5 1.5 0 0 0 1.5 1.5h11A1.5 1.5 0 0 0 19 20V7.5z"/><path d="M14 2.5v5h5"/>',
    text: '<path d="M4 6h16"/><path d="M4 12h16"/><path d="M4 18h10"/>',
    paperclip: '<path d="m21.4 11.6-9.2 9.2a5.5 5.5 0 0 1-7.8-7.8l9.2-9.2a3.7 3.7 0 0 1 5.2 5.2l-9.2 9.2a1.8 1.8 0 0 1-2.6-2.6l8.5-8.5"/>',
    pin: '<path d="M12 17v5"/><path d="M9 10.8V4h6v6.8l3 3.2v2H6v-2z"/>',
    send: '<path d="M14.5 21.5 21.5 2.5 2.5 9.5l8 3.5z"/><path d="m21.5 2.5-11 10"/>',
    settings: '<path d="M10.3 3.5a1.7 1.7 0 0 1 3.4 0l.1.6a1.7 1.7 0 0 0 2.5 1l.5-.3a1.7 1.7 0 0 1 2.4 2.4l-.3.5a1.7 1.7 0 0 0 1 2.5l.6.1a1.7 1.7 0 0 1 0 3.4l-.6.1a1.7 1.7 0 0 0-1 2.5l.3.5a1.7 1.7 0 0 1-2.4 2.4l-.5-.3a1.7 1.7 0 0 0-2.5 1l-.1.6a1.7 1.7 0 0 1-3.4 0l-.1-.6a1.7 1.7 0 0 0-2.5-1l-.5.3a1.7 1.7 0 0 1-2.4-2.4l.3-.5a1.7 1.7 0 0 0-1-2.5l-.6-.1a1.7 1.7 0 0 1 0-3.4l.6-.1a1.7 1.7 0 0 0 1-2.5l-.3-.5a1.7 1.7 0 0 1 2.4-2.4l.5.3a1.7 1.7 0 0 0 2.5-1z"/><circle cx="12" cy="12" r="3"/>',
    download: '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/>',
    x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    eye: '<path d="M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7-9.5-7-9.5-7z"/><circle cx="12" cy="12" r="2.8"/>',
  };

  export const iconNames = Object.keys(paths);
</script>

<script lang="ts">
  let { name, size = 16, stroke = 1.75, class: className = "" }: { name: string; size?: number; stroke?: number; class?: string } =
    $props();
</script>

<svg
  class="icon {className}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true">{@html paths[name] ?? paths.box}</svg
>

<style>
  .icon {
    flex: none;
    display: block;
  }
</style>
