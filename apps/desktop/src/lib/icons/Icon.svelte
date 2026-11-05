<script lang="ts">
  // 20px grid, 1.75 stroke, rounded caps. One map keeps the bundle small.
  const paths: Record<string, string> = {
    home: 'M3.5 9.2 10 3.5l6.5 5.7V16a1 1 0 0 1-1 1H12v-4.5H8V17H4.5a1 1 0 0 1-1-1V9.2Z',
    download: 'M10 3.5v9m0 0-3.5-3.5M10 12.5 13.5 9M4 15.5h12',
    organizer: 'M3 6.5a1.5 1.5 0 0 1 1.5-1.5h3.2l1.6 1.8h5.2A1.5 1.5 0 0 1 16.5 8.3v6.2A1.5 1.5 0 0 1 15 16H4.5A1.5 1.5 0 0 1 3 14.5v-8Z',
    convert: 'M4 7h10.5m0 0-2.5-2.5M14.5 7 12 9.5M16 13H5.5m0 0L8 10.5M5.5 13 8 15.5',
    settings:
      'M10 12.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM10 2.8v1.7M10 15.5v1.7M17.2 10h-1.7M4.5 10H2.8M15.1 4.9l-1.2 1.2M6.1 13.9l-1.2 1.2M15.1 15.1l-1.2-1.2M6.1 6.1 4.9 4.9',
    panel: 'M3.5 5.5a1 1 0 0 1 1-1h11a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-9ZM8 4.5v11',
    check: 'm4.5 10.5 3.5 3.5 7.5-8',
    // a cat face: ears, whisker-less head, closed-smile mouth
    cat: 'M4.2 8.2 4.8 3.4 8.2 5.8Q10 5.4 11.8 5.8L15.2 3.4 15.8 8.2Q16.8 10 16.2 12.8 15 16.6 10 16.6 5 16.6 3.8 12.8 3.2 10 4.2 8.2ZM7.6 10.3h.01M12.4 10.3h.01M8.7 12.6q.65.8 1.3 0 .65.8 1.3 0',
  };
  let { name, size = 20 }: { name: string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 20 20"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name] ?? paths.home} />
</svg>
